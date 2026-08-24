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

    /// Returns true if we have a session token configured.
    pub fn has_session_token(&self) -> bool {
        !self.auth.session_token.is_empty()
    }

    /// Detect the key/auth tier from the session token or legacy API key prefix.
    pub fn key_tier(&self) -> KeyTier {
        KeyTier::from_key(&self.auth.session_token)
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
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyTier {
    Management,
    Server,
    Client,
    Session,
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
            KeyTier::Unknown => "unknown",
        }
    }

    pub fn can_mutate(&self) -> bool {
        matches!(self, KeyTier::Management | KeyTier::Session)
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
}
