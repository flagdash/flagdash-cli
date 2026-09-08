use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub connection: ConnectionConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    /// Device-flow session token (`session_*`). The original CLI credential.
    ///
    /// Superseded by the OAuth fields below but still honoured: it is what
    /// `FLAGDASH_SESSION_TOKEN` sets, what a already-logged-in config holds, and
    /// what CI uses when it cannot open a browser.
    #[serde(default)]
    pub session_token: String,
    #[serde(default)]
    pub user_name: String,
    #[serde(default)]
    pub user_email: String,
    #[serde(default)]
    pub user_role: String,
    #[serde(default)]
    pub token_expires_at: String,

    /// OAuth 2.1 access token (`mcp_at_*`), obtained through the device grant.
    ///
    /// Short-lived — one hour — and refreshed silently, which is why the refresh
    /// token and expiry sit beside it. A blank access token with a live refresh
    /// token is the normal resting state after an hour of inactivity.
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    /// RFC 3339. Compared before every request so a known-expired token is
    /// refreshed proactively instead of costing a 401 round trip first.
    #[serde(default)]
    pub access_expires_at: String,
    /// The dynamically registered client this device authenticates as.
    ///
    /// Registration is once per machine (RFC 7591), so this is cached: losing it
    /// means registering again, which works but leaves an orphan client behind.
    #[serde(default)]
    pub client_id: String,
    /// Space-separated scopes the person actually granted.
    #[serde(default)]
    pub scope: String,

    /// Legacy field: kept for backwards compatibility with existing config files.
    /// If present and session_token is empty, it will be used as a fallback.
    #[serde(default, skip_serializing)]
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
        }
    }
}

fn default_base_url() -> String {
    "https://flagdash.io".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub environment_id: String,
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub environment_name: String,
}

impl AppConfig {
    /// Load config with priority: CLI args > env vars > config file
    pub fn load(
        cli_session_token: Option<&str>,
        cli_base_url: Option<&str>,
        cli_project_id: Option<&str>,
        cli_environment_id: Option<&str>,
    ) -> Result<Self> {
        // Start with config file
        let mut config = Self::load_from_file().unwrap_or_default();

        // Migrate legacy api_key field to session_token
        if config.auth.session_token.is_empty() && !config.auth.api_key.is_empty() {
            config.auth.session_token = std::mem::take(&mut config.auth.api_key);
        }

        // Override with env vars (FLAGDASH_SESSION_TOKEN takes priority, FLAGDASH_API_KEY as fallback)
        if let Ok(token) = std::env::var("FLAGDASH_SESSION_TOKEN") {
            config.auth.session_token = token;
        } else if let Ok(key) = std::env::var("FLAGDASH_API_KEY") {
            config.auth.session_token = key;
        }
        // An access token supplied by the environment is used as-is and never
        // refreshed: whoever exported it owns its lifecycle, and writing a
        // refreshed value back to disk would be a surprising side effect.
        if let Ok(token) = std::env::var("FLAGDASH_ACCESS_TOKEN") {
            config.auth.access_token = token;
            config.auth.refresh_token.clear();
            config.auth.access_expires_at.clear();
        }
        if let Ok(url) = std::env::var("FLAGDASH_BASE_URL") {
            config.connection.base_url = url;
        }
        if let Ok(pid) = std::env::var("FLAGDASH_PROJECT_ID") {
            config.defaults.project_id = pid;
        }
        if let Ok(eid) = std::env::var("FLAGDASH_ENVIRONMENT_ID") {
            config.defaults.environment_id = eid;
        }

        // Override with CLI args
        if let Some(token) = cli_session_token {
            config.auth.session_token = token.to_string();
        }
        if let Some(url) = cli_base_url {
            config.connection.base_url = url.to_string();
        }
        if let Some(pid) = cli_project_id {
            config.defaults.project_id = pid.to_string();
        }
        if let Some(eid) = cli_environment_id {
            config.defaults.environment_id = eid.to_string();
        }

        validate_base_url(&config.connection.base_url)?;

        Ok(config)
    }

    fn load_from_file() -> Result<Self> {
        let path = config_file_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let config: AppConfig =
            toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
        Ok(config)
    }

    /// Save the current config to the config file.
    ///
    /// The file holds the bearer session token, so on Unix it is written with
    /// owner-only permissions (dir 0700, file 0600) to keep other local users
    /// from reading the credential.
    pub fn save(&self) -> Result<()> {
        let path = config_file_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating config dir {}", parent.display()))?;
            restrict_dir_permissions(parent);
        }
        let content = toml::to_string_pretty(self).context("serializing config")?;
        write_private(&path, content.as_bytes())
            .with_context(|| format!("writing config to {}", path.display()))?;
        Ok(())
    }

    /// Returns true if we have any credential configured.
    ///
    /// Named for the field it originally checked; it now answers the broader
    /// question every caller actually meant, so an OAuth-only login is not
    /// mistaken for being signed out.
    pub fn has_session_token(&self) -> bool {
        !self.auth.session_token.is_empty() || self.has_oauth_token()
    }

    /// Whether an OAuth credential is present — either a usable access token or
    /// a refresh token that can mint one.
    pub fn has_oauth_token(&self) -> bool {
        !self.auth.access_token.is_empty() || !self.auth.refresh_token.is_empty()
    }

    /// The bearer token to send, preferring OAuth over the older session token.
    ///
    /// Returns `None` when the access token is absent or expired and a refresh is
    /// required first — callers must not fall back to the session token in that
    /// case, or a stale login would silently act with different scopes than the
    /// one the person most recently granted.
    pub fn bearer_token(&self) -> Option<&str> {
        if !self.auth.access_token.is_empty() && !self.access_token_expired() {
            Some(&self.auth.access_token)
        } else if self.auth.access_token.is_empty()
            && self.auth.refresh_token.is_empty()
            && !self.auth.session_token.is_empty()
        {
            Some(&self.auth.session_token)
        } else {
            None
        }
    }

    /// Whether the access token is past (or within a minute of) its expiry.
    ///
    /// The minute of slack matters: a token that expires mid-flight fails the
    /// request rather than the check, and a retry loop around a 401 is a worse
    /// place to discover it than a proactive refresh.
    pub fn access_token_expired(&self) -> bool {
        if self.auth.access_expires_at.is_empty() {
            // No recorded expiry means the token came from the environment, where
            // its lifecycle is not ours to manage. Treat it as usable.
            return false;
        }

        match chrono::DateTime::parse_from_rfc3339(&self.auth.access_expires_at) {
            Ok(expires_at) => {
                chrono::Utc::now() + chrono::Duration::seconds(60)
                    >= expires_at.with_timezone(&chrono::Utc)
            }
            // An unparseable timestamp is treated as expired: refreshing costs one
            // request, while trusting it costs every request until someone notices.
            Err(_) => true,
        }
    }

    /// Record a freshly issued OAuth token pair.
    pub fn set_oauth_tokens(
        &mut self,
        access_token: String,
        refresh_token: String,
        expires_in: i64,
        scope: String,
    ) {
        self.auth.access_token = access_token;
        self.auth.refresh_token = refresh_token;
        self.auth.scope = scope;
        self.auth.access_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(expires_in))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        // An OAuth login supersedes any older session token, which would
        // otherwise linger with wider scopes than the person just granted.
        self.auth.session_token.clear();
    }

    /// Detect the key/auth tier from whichever credential is in use.
    pub fn key_tier(&self) -> KeyTier {
        if self.has_oauth_token() {
            KeyTier::OAuth
        } else {
            KeyTier::from_key(&self.auth.session_token)
        }
    }

    /// Detect the key/auth tier from the user role stored in config.
    pub fn user_role_tier(&self) -> KeyTier {
        if !self.auth.user_role.is_empty() {
            KeyTier::from_role(&self.auth.user_role)
        } else {
            self.key_tier()
        }
    }

    /// Clear all auth fields (logout).
    pub fn clear_auth(&mut self) {
        self.auth.session_token.clear();
        self.auth.user_name.clear();
        self.auth.user_email.clear();
        self.auth.user_role.clear();
        self.auth.token_expires_at.clear();
        self.auth.api_key.clear();
        self.auth.access_token.clear();
        self.auth.refresh_token.clear();
        self.auth.access_expires_at.clear();
        self.auth.scope.clear();
        // `client_id` deliberately survives a logout: it identifies this
        // installation, not the person, and re-registering on every sign-in
        // would leave an orphan client row behind each time.
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyTier {
    Management,
    Server,
    Client,
    Session,
    /// A token from the OAuth device grant. What it may do is decided by the
    /// scopes the person granted, which the server enforces — so it is treated
    /// as mutating here and refused server-side if the grant was read-only.
    OAuth,
    Unknown,
}

impl KeyTier {
    pub fn from_key(key: &str) -> Self {
        if key.starts_with("management_") {
            KeyTier::Management
        } else if key.starts_with("server_") {
            KeyTier::Server
        } else if key.starts_with("client_") {
            KeyTier::Client
        } else if key.starts_with("session_") {
            KeyTier::Session
        } else if key.starts_with("mcp_at_") {
            KeyTier::OAuth
        } else {
            KeyTier::Unknown
        }
    }

    pub fn from_role(role: &str) -> Self {
        match role {
            "owner" | "admin" => KeyTier::Management,
            "member" | "editor" => KeyTier::Server,
            "viewer" => KeyTier::Client,
            _ => KeyTier::Session,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            KeyTier::Management => "management",
            KeyTier::Server => "server",
            KeyTier::Client => "client",
            KeyTier::Session => "session",
            KeyTier::OAuth => "oauth",
            KeyTier::Unknown => "unknown",
        }
    }

    pub fn can_mutate(&self) -> bool {
        matches!(
            self,
            KeyTier::Management | KeyTier::Session | KeyTier::OAuth
        )
    }
}

/// Reject plaintext-HTTP base URLs pointing at a non-local host, since the
/// bearer session token would be transmitted in cleartext. `https://` is always
/// allowed; `http://` is allowed only for localhost/loopback (local dev).
fn validate_base_url(base_url: &str) -> Result<()> {
    let lower = base_url.trim().to_lowercase();

    if lower.starts_with("https://") {
        return Ok(());
    }

    if let Some(rest) = lower.strip_prefix("http://") {
        // Extract the host, handling bracketed IPv6 literals (e.g. "[::1]:4000").
        let host = if let Some(after) = rest.strip_prefix('[') {
            match after.split_once(']') {
                Some((inner, _)) => format!("[{inner}]"),
                None => rest.to_string(),
            }
        } else {
            rest.split(['/', ':', '?']).next().unwrap_or("").to_string()
        };

        let is_local = matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "[::1]")
            || host.ends_with(".localhost");

        if is_local {
            return Ok(());
        }

        anyhow::bail!(
            "Refusing to use insecure base URL {base_url:?}: the session token would be sent \
             over plaintext HTTP. Use https:// (http:// is only allowed for localhost)."
        );
    }

    // No scheme or an unexpected scheme — leave as-is; the HTTP client will
    // surface a clearer error when it tries to connect.
    Ok(())
}

/// Write a file with owner-only permissions (0600) on Unix. The file is
/// created with the restrictive mode from the start (via OpenOptions) so the
/// token is never briefly readable at a wider mode. On non-Unix platforms this
/// falls back to a plain write.
#[cfg(unix)]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // Ensure mode is 0600 even if the file already existed with wider perms.
    let perms = std::fs::Permissions::from_mode(0o600);
    file.set_permissions(perms)?;
    file.write_all(bytes)
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

/// Best-effort tighten of the config directory to owner-only (0700) on Unix.
#[cfg(unix)]
fn restrict_dir_permissions(dir: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn restrict_dir_permissions(_dir: &std::path::Path) {}

/// Returns the platform-appropriate config file path.
pub fn config_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("could not determine config directory")?
        .join("flagdash");
    Ok(config_dir.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_tier_detection() {
        assert_eq!(KeyTier::from_key("management_abc123"), KeyTier::Management);
        assert_eq!(KeyTier::from_key("server_abc123"), KeyTier::Server);
        assert_eq!(KeyTier::from_key("client_abc123"), KeyTier::Client);
        assert_eq!(KeyTier::from_key("session_abc123"), KeyTier::Session);
        assert_eq!(KeyTier::from_key("unknown_abc123"), KeyTier::Unknown);
        assert_eq!(KeyTier::from_key(""), KeyTier::Unknown);
    }

    #[test]
    fn test_key_tier_from_role() {
        assert_eq!(KeyTier::from_role("owner"), KeyTier::Management);
        assert_eq!(KeyTier::from_role("admin"), KeyTier::Management);
        assert_eq!(KeyTier::from_role("member"), KeyTier::Server);
        assert_eq!(KeyTier::from_role("editor"), KeyTier::Server);
        assert_eq!(KeyTier::from_role("viewer"), KeyTier::Client);
        assert_eq!(KeyTier::from_role("something"), KeyTier::Session);
    }

    #[test]
    fn test_key_tier_can_mutate() {
        assert!(KeyTier::Management.can_mutate());
        assert!(KeyTier::Session.can_mutate());
        assert!(!KeyTier::Server.can_mutate());
        assert!(!KeyTier::Client.can_mutate());
    }

    #[test]
    fn test_validate_base_url() {
        // https is always allowed
        assert!(validate_base_url("https://flagdash.io").is_ok());
        assert!(validate_base_url("https://self-hosted.example.com").is_ok());

        // http allowed only for localhost/loopback
        assert!(validate_base_url("http://localhost:4000").is_ok());
        assert!(validate_base_url("http://127.0.0.1:4000").is_ok());
        assert!(validate_base_url("http://[::1]:4000").is_ok());

        // http to a remote host is rejected (would leak the token in cleartext)
        assert!(validate_base_url("http://flagdash.io").is_err());
        assert!(validate_base_url("http://192.168.1.50:4000").is_err());
        assert!(validate_base_url("HTTP://Example.COM").is_err());
    }

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.connection.base_url, "https://flagdash.io");
        assert!(config.auth.session_token.is_empty());
        assert!(!config.has_session_token());
    }

    #[test]
    fn test_clear_auth() {
        let mut config = AppConfig::default();
        config.auth.session_token = "session_test".to_string();
        config.auth.user_name = "Test User".to_string();
        config.auth.user_email = "test@example.com".to_string();
        config.auth.user_role = "admin".to_string();
        config.auth.token_expires_at = "2026-03-01T00:00:00Z".to_string();

        config.clear_auth();

        assert!(config.auth.session_token.is_empty());
        assert!(config.auth.user_name.is_empty());
        assert!(config.auth.user_email.is_empty());
        assert!(config.auth.user_role.is_empty());
        assert!(config.auth.token_expires_at.is_empty());
    }

    // ── OAuth device-grant credentials ───────────────────────────────
    //
    // The expiry logic here decides whether every request carries a live token
    // or a dead one, and it is the kind of thing that fails silently: a token
    // treated as valid past its expiry produces 401s on unrelated screens.

    fn oauth_config(expires_in_secs: i64) -> AppConfig {
        let mut config = AppConfig::default();
        config.set_oauth_tokens(
            "mcp_at_abc".to_string(),
            "mcp_rt_def".to_string(),
            expires_in_secs,
            "mcp:read mcp:write".to_string(),
        );
        config
    }

    #[test]
    fn set_oauth_tokens_records_an_absolute_expiry() {
        let config = oauth_config(3600);

        assert_eq!(config.auth.access_token, "mcp_at_abc");
        assert_eq!(config.auth.refresh_token, "mcp_rt_def");
        assert_eq!(config.auth.scope, "mcp:read mcp:write");
        assert!(!config.auth.access_expires_at.is_empty());
        assert!(!config.access_token_expired());
    }

    #[test]
    fn an_oauth_login_clears_any_older_session_token() {
        // Otherwise a session token from a previous login lingers with wider
        // scopes than the person just granted.
        let mut config = AppConfig::default();
        config.auth.session_token = "session_old".to_string();

        config.set_oauth_tokens(
            "mcp_at_new".to_string(),
            "mcp_rt_new".to_string(),
            3600,
            String::new(),
        );

        assert!(config.auth.session_token.is_empty());
    }

    #[test]
    fn a_token_inside_the_expiry_slack_counts_as_expired() {
        // 30 seconds left is not enough: a token that expires mid-flight fails
        // the request rather than the check.
        let config = oauth_config(30);
        assert!(config.access_token_expired());
    }

    #[test]
    fn a_past_expiry_counts_as_expired() {
        let config = oauth_config(-10);
        assert!(config.access_token_expired());
    }

    #[test]
    fn an_unparseable_expiry_counts_as_expired() {
        // Refreshing costs one request; trusting it costs every request until
        // somebody notices.
        let mut config = oauth_config(3600);
        config.auth.access_expires_at = "not a timestamp".to_string();
        assert!(config.access_token_expired());
    }

    #[test]
    fn a_token_with_no_recorded_expiry_is_usable() {
        // That is the environment-supplied case, where the lifecycle is not ours.
        let mut config = AppConfig::default();
        config.auth.access_token = "mcp_at_from_env".to_string();

        assert!(!config.access_token_expired());
        assert_eq!(config.bearer_token(), Some("mcp_at_from_env"));
    }

    #[test]
    fn bearer_token_prefers_oauth_over_a_session_token() {
        let mut config = oauth_config(3600);
        config.auth.session_token = "session_old".to_string();

        assert_eq!(config.bearer_token(), Some("mcp_at_abc"));
    }

    #[test]
    fn bearer_token_withholds_an_expired_access_token() {
        // Returning the session token here would silently act with different
        // scopes than the OAuth grant the person most recently approved.
        let mut config = oauth_config(-10);
        config.auth.session_token = "session_old".to_string();

        assert_eq!(config.bearer_token(), None);
    }

    #[test]
    fn bearer_token_falls_back_to_a_session_token_when_there_is_no_oauth_login() {
        let mut config = AppConfig::default();
        config.auth.session_token = "session_only".to_string();

        assert_eq!(config.bearer_token(), Some("session_only"));
        assert!(config.has_session_token());
        assert!(!config.has_oauth_token());
    }

    #[test]
    fn an_oauth_login_counts_as_signed_in() {
        let config = oauth_config(3600);

        assert!(config.has_session_token());
        assert!(config.has_oauth_token());
        assert_eq!(config.key_tier(), KeyTier::OAuth);
    }

    #[test]
    fn a_refresh_token_alone_still_counts_as_signed_in() {
        // The resting state after an hour idle: the access token is gone, the
        // refresh token restores it. Reporting "signed out" here would present a
        // login screen to someone who is signed in.
        let mut config = AppConfig::default();
        config.auth.refresh_token = "mcp_rt_only".to_string();

        assert!(config.has_oauth_token());
        assert!(config.has_session_token());
        // ...but there is nothing to send until it is refreshed.
        assert_eq!(config.bearer_token(), None);
    }

    #[test]
    fn clear_auth_drops_oauth_credentials_but_keeps_the_client_id() {
        let mut config = oauth_config(3600);
        config.auth.client_id = "installation-client".to_string();

        config.clear_auth();

        assert!(config.auth.access_token.is_empty());
        assert!(config.auth.refresh_token.is_empty());
        assert!(config.auth.access_expires_at.is_empty());
        assert!(config.auth.scope.is_empty());
        // The client_id identifies the installation, not the person — losing it
        // means re-registering and leaving an orphan client behind.
        assert_eq!(config.auth.client_id, "installation-client");
    }

    #[test]
    fn an_oauth_access_token_is_recognised_by_prefix() {
        assert_eq!(KeyTier::from_key("mcp_at_abc123"), KeyTier::OAuth);
        assert!(KeyTier::OAuth.can_mutate());
        assert_eq!(KeyTier::OAuth.label(), "oauth");
    }
}
