//! Secrets: `flagdash secrets list | show | create | set | versions | restore |
//! approve | delete | recover`.
//!
//! Three rules shape this module, and they are the whole design:
//!
//! 1. **A listing never carries a value.** The management API has no action that
//!    returns one, so there is nothing here to accidentally print.
// 2. **Retrieval needs a different credential.** `fetch` reads
//!    `GET /api/v1/server/secrets/:key` with a project-scoped `sk_` key holding
//!    `secrets:read`, supplied per invocation. A login session token cannot do
//!    it, and the `sk_` key is never written to the config file — so a
//!    developer's everyday `flagdash` cannot print a production credential, and
//!    CI gets exactly the one capability it needs.
//! 3. **Nothing retrieved is ever written to disk unless asked.** No value
//!    passes through `AppConfig`, the log file, or any cache; `--out` writes the
//!    one file the caller named, at mode 0600.
//!
//! Values going *in* are read from a file or stdin rather than an argument, so a
//! credential does not land in shell history or a process listing.

use crate::api::client::ApiClient;
use crate::api::types::{
    CreateSecretRequest, ManagedSecret, ReplaceSecretRequest, RestoreSecretRequest,
};
use crate::config::AppConfig;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Where a new secret value comes from. Never an argv string.
#[derive(Debug, Clone, Default)]
pub struct ValueSource {
    /// Read the value from this file.
    pub file: Option<PathBuf>,
    /// Read the value from stdin.
    pub stdin: bool,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub project_id: String,
    pub environment_id: String,
}

fn scope(config: &AppConfig) -> Result<Scope> {
    let project_id = config.defaults.project_id.clone();
    let environment_id = config.defaults.environment_id.clone();

    if project_id.is_empty() || environment_id.is_empty() {
        bail!(
            "a project and environment are required — pass --project-id and --environment-id, \
             or select them once in the TUI"
        );
    }

    Ok(Scope {
        project_id,
        environment_id,
    })
}

fn client(config: &AppConfig) -> Result<ApiClient> {
    let token = config
        .bearer_token()
        .context("not logged in (or the session expired) — run `flagdash login` and try again")?;

    Ok(ApiClient::new(&config.connection.base_url, token))
}

/// Read the plaintext for a create or replace.
///
/// A `--value` flag is deliberately not offered: an argument is visible in shell
/// history and in every process listing on the machine.
fn read_value(format: &str, source: &ValueSource) -> Result<serde_json::Value> {
    let raw = match (&source.file, source.stdin) {
        (Some(path), _) => std::fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?,
        (None, true) => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .context("could not read the value from stdin")?;
            buffer
        }
        (None, false) => bail!("pass --from-file <path> or --stdin to supply the value"),
    };

    parse_value(format, &raw)
}

/// Turn raw input into the JSON body the API expects.
fn parse_value(format: &str, raw: &str) -> Result<serde_json::Value> {
    match format {
        "json" => {
            let value: serde_json::Value =
                serde_json::from_str(raw.trim()).context("the value is not valid JSON")?;
            if !value.is_object() && !value.is_array() {
                bail!("a json secret must be an object or an array");
            }
            Ok(value)
        }
        "string" => {
            // A trailing newline is an artefact of `echo` and of most editors; a
            // credential with one appended is the classic silent auth failure.
            // Only the trailing end is trimmed: a credential whose first
            // character really is a space must survive intact.
            let trimmed = raw.trim_end_matches(['\n', '\r']);
            if trimmed.is_empty() {
                bail!("the value is empty");
            }
            Ok(serde_json::Value::String(trimmed.to_string()))
        }
        other => bail!("unknown secret format {other:?} — expected string or json"),
    }
}

fn status(secret: &ManagedSecret) -> &'static str {
    if secret.deleted_at.is_some() {
        "deleted"
    } else if secret.approval_pending {
        "pending approval"
    } else if secret.has_value {
        "active"
    } else {
        "no value"
    }
}

pub async fn list(config: &AppConfig) -> Result<()> {
    let scope = scope(config)?;
    let secrets = client(config)?
        .list_secrets(&scope.project_id, &scope.environment_id)
        .await?;

    if secrets.is_empty() {
        println!("No secrets in this environment.");
        return Ok(());
    }

    let width = secrets
        .iter()
        .map(|s| s.key.len())
        .max()
        .unwrap_or(3)
        .max(3);
    println!("{:<width$}  {:<6}  STATUS", "KEY", "FORMAT", width = width);

    for secret in &secrets {
        println!(
            "{:<width$}  {:<6}  {}",
            secret.key,
            secret.format,
            status(secret),
            width = width
        );
    }

    println!(
        "\nValues are not shown here and cannot be read from the CLI. Read one from your \
         backend with an sk_ key holding secrets:read."
    );
    Ok(())
}

pub async fn show(config: &AppConfig, key: &str) -> Result<()> {
    let scope = scope(config)?;
    let secret = client(config)?
        .get_secret_metadata(key, &scope.project_id, &scope.environment_id)
        .await?;

    println!("key         {}", secret.key);
    println!("name        {}", secret.name);
    if !secret.description.is_empty() {
        println!("description {}", secret.description);
    }
    println!("format      {}", secret.format);
    println!("status      {}", status(&secret));
    println!(
        "version     {}",
        secret.current_version_id.as_deref().unwrap_or("—")
    );
    if let Some(pending) = &secret.pending_version_id {
        println!("pending     {pending}");
    }
    println!("updated     {}", secret.updated_at);
    Ok(())
}

pub async fn create(
    config: &AppConfig,
    key: &str,
    name: Option<&str>,
    description: &str,
    format: &str,
    source: &ValueSource,
) -> Result<()> {
    let scope = scope(config)?;
    let value = read_value(format, source)?;

    let request = CreateSecretRequest {
        project_id: scope.project_id.clone(),
        environment_id: scope.environment_id.clone(),
        key: key.to_string(),
        name: name.unwrap_or(key).to_string(),
        description: description.to_string(),
        format: format.to_string(),
        value,
    };

    let secret = client(config)?.create_secret(&request).await?;
    println!("Created secret '{}' ({}).", secret.key, secret.format);
    Ok(())
}

pub async fn set(config: &AppConfig, key: &str, source: &ValueSource) -> Result<()> {
    let scope = scope(config)?;
    let api = client(config)?;

    // The expected version is read rather than asked for: a replacement that
    // raced another writer must be refused, not merged.
    let current = api
        .get_secret_metadata(key, &scope.project_id, &scope.environment_id)
        .await?;

    let value = read_value(&current.format, source)?;

    let version = api
        .replace_secret(
            key,
            &scope.project_id,
            &scope.environment_id,
            &ReplaceSecretRequest {
                value,
                expected_version_id: current.current_version_id.clone(),
            },
        )
        .await?;

    println!("Replaced '{key}' — new version {}.", version.version_id);
    if version.pending {
        println!("Awaiting approval by a different user before it takes effect.");
    }
    Ok(())
}

pub async fn versions(config: &AppConfig, key: &str) -> Result<()> {
    let scope = scope(config)?;
    let history = client(config)?
        .secret_versions(key, &scope.project_id, &scope.environment_id)
        .await?;

    if history.versions.is_empty() {
        println!("No versions recorded for '{key}'.");
        return Ok(());
    }

    for version in &history.versions {
        let marker = if version.current {
            "current"
        } else if version.pending {
            "pending"
        } else {
            ""
        };
        println!("{}  {}  {}", version.version_id, version.created_at, marker);
    }
    Ok(())
}

pub async fn restore(config: &AppConfig, key: &str, version_id: &str) -> Result<()> {
    let scope = scope(config)?;
    let api = client(config)?;

    let current = api
        .get_secret_metadata(key, &scope.project_id, &scope.environment_id)
        .await?;

    let version = api
        .restore_secret_version(
            key,
            version_id,
            &scope.project_id,
            &scope.environment_id,
            &RestoreSecretRequest {
                expected_version_id: current.current_version_id.clone(),
            },
        )
        .await?;

    println!(
        "Restored '{key}' from {version_id} as new version {}.",
        version.version_id
    );
    Ok(())
}

pub async fn approve(config: &AppConfig, key: &str) -> Result<()> {
    let scope = scope(config)?;
    let version = client(config)?
        .approve_secret(key, &scope.project_id, &scope.environment_id)
        .await?;

    println!("Approved '{key}' version {}.", version.version_id);
    Ok(())
}

pub async fn delete(config: &AppConfig, key: &str) -> Result<()> {
    let scope = scope(config)?;
    client(config)?
        .delete_secret(key, &scope.project_id, &scope.environment_id)
        .await?;

    println!("Deleted '{key}'. Recoverable for 7 days, then the ciphertext is purged.");
    Ok(())
}

pub async fn recover(config: &AppConfig, key: &str) -> Result<()> {
    let scope = scope(config)?;
    client(config)?
        .recover_secret(key, &scope.project_id, &scope.environment_id)
        .await?;

    println!("Recovered '{key}'.");
    Ok(())
}

// ── Retrieval, for CI ────────────────────────────────────────────────────────

/// How fetched values are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// The bare value. One key only — there is nothing to disambiguate with.
    Raw,
    /// `NAME='value'` lines, ready for `source` or `>> $GITHUB_ENV`.
    Env,
    /// A JSON object keyed by secret key.
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "raw" => Ok(OutputFormat::Raw),
            "env" => Ok(OutputFormat::Env),
            "json" => Ok(OutputFormat::Json),
            other => Err(format!(
                "unknown format {other:?} — expected raw, env or json"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FetchOptions {
    pub keys: Vec<String>,
    pub sdk_key: Option<String>,
    pub format: OutputFormat,
    pub out: Option<PathBuf>,
    pub no_mask: bool,
}

/// Fetch one or more secrets for a CI step.
///
/// The credential is deliberately *not* the login token: this reads the server
/// tier, which only a project-scoped `sk_` key can. That is what makes a CI
/// secret-fetch job safe to grant — the key it holds can read secrets in one
/// environment and do nothing else.
pub async fn fetch(config: &AppConfig, options: &FetchOptions) -> Result<()> {
    if options.keys.is_empty() {
        bail!("name at least one secret to fetch");
    }

    if options.format == OutputFormat::Raw && options.keys.len() > 1 {
        bail!("--format raw prints a bare value, so it takes exactly one key; use env or json");
    }

    let sdk_key = resolve_sdk_key(options)?;
    let base_url = config.connection.base_url.trim_end_matches('/').to_string();
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("could not build the HTTP client")?;

    // BTreeMap so `env` and `json` output is ordered, and a CI diff of the
    // rendered file is stable rather than reshuffling on every run.
    let mut values: BTreeMap<String, serde_json::Value> = BTreeMap::new();

    for key in &options.keys {
        let value = fetch_one(&http, &base_url, &sdk_key, key).await?;
        if !options.no_mask {
            mask(&value);
        }
        values.insert(key.clone(), value);
    }

    let rendered = render(options.format, &values)?;

    match &options.out {
        Some(path) => {
            write_private(path, &rendered)
                .with_context(|| format!("could not write {}", path.display()))?;
            // The path, never the contents.
            eprintln!("Wrote {} secret(s) to {}", values.len(), path.display());
        }
        None => print!("{rendered}"),
    }

    Ok(())
}

/// The `sk_` key, from the flag or the environment — never from the config file.
///
/// Reading it from `AppConfig` would mean a developer's saved login could print
/// production credentials, and that a `flagdash login` on a shared machine
/// widens what every later command can do.
fn resolve_sdk_key(options: &FetchOptions) -> Result<String> {
    let key = options
        .sdk_key
        .clone()
        .or_else(|| std::env::var("FLAGDASH_SDK_KEY").ok())
        .filter(|value| !value.trim().is_empty())
        .context(
            "a project-scoped SDK key is required — pass --sdk-key or set FLAGDASH_SDK_KEY. \
             It must hold the secrets:read scope; your login token cannot read secret values.",
        )?;

    let key = key.trim().to_string();

    if !key.starts_with("sk_") {
        bail!(
            "that is not a project-scoped SDK key — secret retrieval needs an sk_ key bound to \
             one project and environment, not a session or personal access token"
        );
    }

    Ok(key)
}

async fn fetch_one(
    http: &reqwest::Client,
    base_url: &str,
    sdk_key: &str,
    key: &str,
) -> Result<serde_json::Value> {
    let url = format!("{base_url}/api/v1/server/secrets/{}", urlencode(key));

    let response = http
        .get(&url)
        .bearer_auth(sdk_key)
        .send()
        .await
        .with_context(|| format!("could not reach FlagDash to fetch '{key}'"))?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);

    if !status.is_success() {
        // Surface the server's own message: it distinguishes "no such secret"
        // from "this key lacks the scope" from "encryption is unavailable", and
        // a CI operator needs to know which.
        let message = body
            .get("message")
            .and_then(|m| m.as_str())
            .or_else(|| body.get("error").and_then(|e| e.as_str()))
            .unwrap_or("no detail returned");
        bail!("fetching '{key}' failed ({status}): {message}");
    }

    body.get("secret")
        .and_then(|secret| secret.get("value"))
        .cloned()
        .with_context(|| format!("FlagDash returned no value for '{key}'"))
}

/// Ask the CI runner to redact the value from its own logs.
///
/// Best-effort and provider-specific: GitHub Actions honours `::add-mask::`, and
/// on anything else this is a no-op rather than a guess. A multi-line value is
/// masked line by line, because the directive is per line.
fn mask(value: &serde_json::Value) {
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        return;
    }

    let text = match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    };

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        println!("::add-mask::{line}");
    }
}

fn render(format: OutputFormat, values: &BTreeMap<String, serde_json::Value>) -> Result<String> {
    match format {
        OutputFormat::Raw => {
            let value = values.values().next().expect("one key was required");
            Ok(match value {
                serde_json::Value::String(text) => format!("{text}\n"),
                other => format!("{other}\n"),
            })
        }

        OutputFormat::Env => {
            let mut out = String::new();
            for (key, value) in values {
                let text = match value {
                    serde_json::Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                out.push_str(&format!("{}={}\n", env_name(key), shell_quote(&text)));
            }
            Ok(out)
        }

        OutputFormat::Json => {
            Ok(serde_json::to_string_pretty(values).context("could not encode the result")? + "\n")
        }
    }
}

/// `stripe-secret-key` → `STRIPE_SECRET_KEY`.
fn env_name(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Single-quote for POSIX shells, which is the only quoting that is safe for an
/// arbitrary credential: inside single quotes nothing is special, and an
/// embedded quote is closed, escaped and reopened.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(unix)]
fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;

    file.write_all(contents.as_bytes())
}

#[cfg(not(unix))]
fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    std::fs::write(path, contents)
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_value_loses_only_its_trailing_newline() {
        assert_eq!(
            parse_value("string", "sk_live_abc123\n").unwrap(),
            serde_json::Value::String("sk_live_abc123".to_string())
        );
    }

    #[test]
    fn leading_whitespace_in_a_string_secret_is_preserved() {
        // Trimming both ends would silently corrupt a credential whose first
        // character really is a space.
        assert_eq!(
            parse_value("string", "  padded  \n").unwrap(),
            serde_json::Value::String("  padded  ".to_string())
        );
    }

    #[test]
    fn a_json_secret_must_be_an_object_or_array() {
        assert!(parse_value("json", "\"just-a-string\"").is_err());
        assert!(parse_value("json", "42").is_err());
    }

    #[test]
    fn a_json_secret_round_trips() {
        let value = parse_value("json", r#"{"client_email": "svc@example.com"}"#).unwrap();
        assert_eq!(value["client_email"], "svc@example.com");
    }

    #[test]
    fn a_json_array_is_accepted() {
        assert!(parse_value("json", "[1, 2, 3]").is_ok());
    }

    #[test]
    fn an_empty_string_value_is_refused() {
        assert!(parse_value("string", "\n").is_err());
        assert!(parse_value("string", "").is_err());
    }

    #[test]
    fn malformed_json_is_refused_rather_than_stored_as_text() {
        assert!(parse_value("json", "{not json").is_err());
    }

    #[test]
    fn an_unknown_format_is_refused() {
        assert!(parse_value("number", "1").is_err());
    }

    #[test]
    fn a_value_must_come_from_a_file_or_stdin() {
        assert!(read_value("string", &ValueSource::default()).is_err());
    }

    // ── fetch ────────────────────────────────────────────────────────────

    fn values(pairs: &[(&str, serde_json::Value)]) -> BTreeMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn an_sdk_key_is_required_and_must_be_project_scoped() {
        let base = FetchOptions {
            keys: vec!["k".into()],
            sdk_key: None,
            format: OutputFormat::Raw,
            out: None,
            no_mask: true,
        };

        // A session or personal access token must be refused rather than sent:
        // it cannot read the server tier, and the failure should say why.
        for token in ["session_abc", "pat_abc", "mcp_at_abc"] {
            let options = FetchOptions {
                sdk_key: Some(token.into()),
                ..base.clone()
            };
            assert!(
                resolve_sdk_key(&options).is_err(),
                "{token} should be refused"
            );
        }

        let options = FetchOptions {
            sdk_key: Some("  sk_live_abc  ".into()),
            ..base
        };
        assert_eq!(resolve_sdk_key(&options).unwrap(), "sk_live_abc");
    }

    #[test]
    fn env_output_quotes_values_a_shell_would_otherwise_interpret() {
        let rendered = render(
            OutputFormat::Env,
            &values(&[("stripe-secret-key", serde_json::json!("a b$c`d\"e"))]),
        )
        .unwrap();

        assert_eq!(rendered, "STRIPE_SECRET_KEY='a b$c`d\"e'\n");
    }

    #[test]
    fn env_output_escapes_an_embedded_single_quote() {
        // The one character single-quoting cannot contain. Getting this wrong
        // ends the quote early and hands the rest of a credential to the shell.
        let rendered = render(
            OutputFormat::Env,
            &values(&[("k", serde_json::json!("it's"))]),
        )
        .unwrap();

        assert_eq!(rendered, "K='it'\\''s'\n");
    }

    #[test]
    fn env_names_are_upper_snake_case() {
        assert_eq!(env_name("stripe-secret-key"), "STRIPE_SECRET_KEY");
        assert_eq!(env_name("db.password"), "DB_PASSWORD");
    }

    #[test]
    fn env_output_is_ordered_so_a_ci_diff_is_stable() {
        let rendered = render(
            OutputFormat::Env,
            &values(&[
                ("zulu", serde_json::json!("z")),
                ("alpha", serde_json::json!("a")),
            ]),
        )
        .unwrap();

        assert_eq!(rendered, "ALPHA='a'\nZULU='z'\n");
    }

    #[test]
    fn raw_output_is_the_bare_value_with_no_key_or_quoting() {
        let rendered = render(
            OutputFormat::Raw,
            &values(&[("k", serde_json::json!("sk_live_abc"))]),
        )
        .unwrap();

        assert_eq!(rendered, "sk_live_abc\n");
    }

    #[test]
    fn json_output_preserves_a_document_secret() {
        let rendered = render(
            OutputFormat::Json,
            &values(&[(
                "gcp",
                serde_json::json!({"client_email": "svc@example.com"}),
            )]),
        )
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["gcp"]["client_email"], "svc@example.com");
    }

    #[test]
    fn a_key_with_a_slash_cannot_escape_its_path_segment() {
        assert_eq!(urlencode("a/../b"), "a%2F..%2Fb");
        assert_eq!(urlencode("plain-key_1.0~"), "plain-key_1.0~");
    }
}
