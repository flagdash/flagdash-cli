//! GitOps: `flagdash plan`, `flagdash apply`, `flagdash gitops export`.
//!
//! The CLI is deliberately thin. It reads YAML, sends JSON, and renders whatever
//! the server says would change — the diffing itself lives on the server so that
//! every client sees an identical plan from one implementation.
//!
//! Exit codes are the integration point, because they are the one thing every CI
//! on every forge understands without a plugin:
//!
//! | code | meaning              |
//! |------|----------------------|
//! | 0    | no changes           |
//! | 2    | changes pending      |
//! | 1    | error                |

use crate::config::AppConfig;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const EXIT_NO_CHANGES: i32 = 0;
pub const EXIT_CHANGES: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
    Markdown,
}

impl std::str::FromStr for Format {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "text" => Ok(Format::Text),
            "json" => Ok(Format::Json),
            "markdown" | "md" => Ok(Format::Markdown),
            other => Err(format!(
                "unknown format {other:?} — expected text, json or markdown"
            )),
        }
    }
}

#[derive(Debug, Serialize)]
struct SyncSource {
    repository: String,
    #[serde(rename = "ref")]
    git_ref: String,
    commit_sha: String,
    author: String,
}

#[derive(Debug, Serialize)]
struct SyncRequest<'a> {
    project_id: &'a str,
    document: serde_json::Value,
    dry_run: bool,
    prune: bool,
    accept_drift: bool,
    source: SyncSource,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    plan: Plan,
    applied: bool,
    #[serde(default)]
    results: Option<Results>,
}

#[derive(Debug, Deserialize)]
struct Plan {
    changes: Vec<Change>,
    summary: Summary,
}

#[derive(Debug, Deserialize)]
struct Summary {
    create: u32,
    update: u32,
    delete: u32,
    drift: u32,
}

impl Summary {
    fn total(&self) -> u32 {
        self.create + self.update + self.delete + self.drift
    }
}

#[derive(Debug, Deserialize)]
struct Change {
    action: String,
    resource: String,
    key: String,
    #[serde(default)]
    environment: Option<String>,
    #[serde(default)]
    diff: serde_json::Value,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Results {
    #[serde(default)]
    applied: Vec<serde_json::Value>,
    #[serde(default)]
    failed: Vec<serde_json::Value>,
}

pub struct Options {
    pub file: Option<PathBuf>,
    pub format: Format,
    pub prune: bool,
    pub accept_drift: bool,
    pub repository: Option<String>,
    pub exit_zero: bool,
}

/// `flagdash plan` — what would change, writing nothing.
pub async fn plan(config: &AppConfig, options: &Options) -> Result<i32> {
    run(config, options, true).await
}

/// `flagdash apply` — make it so.
pub async fn apply(config: &AppConfig, options: &Options) -> Result<i32> {
    run(config, options, false).await
}

async fn run(config: &AppConfig, options: &Options, dry_run: bool) -> Result<i32> {
    require_credentials(config)?;

    let document = load_document(options.file.as_deref())?;
    let source = detect_source(options.repository.clone(), !dry_run)?;

    let base_url = config.connection.base_url.trim_end_matches('/');
    let response = reqwest::Client::new()
        .post(format!("{base_url}/api/v1/manage/sync"))
        .bearer_auth(&config.auth.session_token)
        .json(&SyncRequest {
            project_id: &config.defaults.project_id,
            document,
            dry_run,
            prune: options.prune,
            accept_drift: options.accept_drift,
            source,
        })
        .send()
        .await
        .context("sending the document to FlagDash")?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        report_failure(status, &body);
        bail!("FlagDash rejected the sync");
    }

    let parsed: SyncResponse =
        serde_json::from_str(&body).context("reading the plan FlagDash returned")?;

    match options.format {
        Format::Json => println!("{body}"),
        Format::Text => print_text(&parsed, dry_run),
        Format::Markdown => print_markdown(&parsed, dry_run),
    }

    if let Some(results) = &parsed.results {
        if !results.failed.is_empty() {
            bail!("{} resource(s) failed to apply", results.failed.len());
        }
    }

    if options.exit_zero || parsed.summary_total() == 0 || !dry_run {
        Ok(EXIT_NO_CHANGES)
    } else {
        Ok(EXIT_CHANGES)
    }
}

impl SyncResponse {
    fn summary_total(&self) -> u32 {
        self.plan.summary.total()
    }
}

/// `flagdash gitops export` — the project as a document, so an existing project
/// can be adopted by committing the result rather than hand-writing it.
pub async fn export(config: &AppConfig) -> Result<()> {
    require_credentials(config)?;

    let base_url = config.connection.base_url.trim_end_matches('/');
    let response = reqwest::Client::new()
        .get(format!(
            "{base_url}/api/v1/manage/sync/export?project_id={}",
            config.defaults.project_id
        ))
        .bearer_auth(&config.auth.session_token)
        .send()
        .await
        .context("requesting the export")?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        report_failure(status, &body);
        bail!("FlagDash rejected the export");
    }

    let payload: serde_json::Value =
        serde_json::from_str(&body).context("reading the exported document")?;
    let document = payload.get("document").unwrap_or(&payload);

    println!(
        "# yaml-language-server: $schema=https://flagdash.io/schemas/flagdash-v1.json\n{}",
        serde_yaml::to_string(document).context("rendering the document as YAML")?
    );
    Ok(())
}

// ── input ────────────────────────────────────────────────────────────────────

/// Reads `flagdash.yaml`, or every file in `flagdash.d/` merged in filename
/// order. The directory layout exists so that two people changing two different
/// flags never touch the same file.
fn load_document(explicit: Option<&Path>) -> Result<serde_json::Value> {
    if let Some(path) = explicit {
        return read_yaml(path);
    }

    let single = Path::new("flagdash.yaml");
    let single_yml = Path::new("flagdash.yml");
    let directory = Path::new("flagdash.d");

    if single.exists() {
        read_yaml(single)
    } else if single_yml.exists() {
        read_yaml(single_yml)
    } else if directory.is_dir() {
        read_directory(directory)
    } else {
        bail!(
            "no flagdash.yaml or flagdash.d/ found in {}. \
             Run `flagdash gitops export > flagdash.yaml` to start from what you already have.",
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".into())
        )
    }
}

fn read_yaml(path: &Path) -> Result<serde_json::Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    serde_yaml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn read_directory(directory: &Path) -> Result<serde_json::Value> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory)
        .with_context(|| format!("reading {}", directory.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yaml") | Some("yml")
            )
        })
        .collect();

    paths.sort();

    if paths.is_empty() {
        bail!("{} contains no .yaml or .yml files", directory.display());
    }

    let mut merged = serde_json::Map::new();

    for path in &paths {
        let document = read_yaml(path)?;
        let Some(object) = document.as_object() else {
            bail!("{} must contain a mapping at the top level", path.display());
        };

        for (key, value) in object {
            match (merged.get_mut(key), value) {
                // `flags:` and `segments:` merge across files; a duplicate key
                // in two files is a genuine conflict and says so rather than
                // silently letting the last file win.
                (
                    Some(serde_json::Value::Object(existing)),
                    serde_json::Value::Object(incoming),
                ) => {
                    for (inner_key, inner_value) in incoming {
                        if existing.contains_key(inner_key) {
                            bail!(
                                "{inner_key:?} is defined in more than one file under {} (see {})",
                                directory.display(),
                                path.display()
                            );
                        }
                        existing.insert(inner_key.clone(), inner_value.clone());
                    }
                }
                _ => {
                    merged.insert(key.clone(), value.clone());
                }
            }
        }
    }

    Ok(serde_json::Value::Object(merged))
}

fn require_credentials(config: &AppConfig) -> Result<()> {
    if config.auth.session_token.is_empty() {
        bail!("authentication is required; sign in or set FLAGDASH_API_KEY");
    }
    if config.defaults.project_id.is_empty() {
        bail!("a project is required; pass --project-id, set FLAGDASH_PROJECT_ID, or add `project:` to the document");
    }
    Ok(())
}

// ── git ──────────────────────────────────────────────────────────────────────

/// Where this document came from. `repository` is what records ownership, so an
/// apply cannot proceed without one.
///
/// Running outside a git checkout used to send an empty string here, which the
/// server accepted and then stored as no owner at all — the apply looked like it
/// worked while leaving every flag unmanaged. Fail here instead, where the
/// person can see why.
fn detect_source(
    repository_override: Option<String>,
    require_repository: bool,
) -> Result<SyncSource> {
    let repository = repository_override
        .or_else(detect_repository)
        .unwrap_or_default();

    if require_repository && repository.trim().is_empty() {
        bail!(
            "could not work out which repository this is, and applying needs one to record \
             ownership. Run from a git checkout with an `origin` remote, or pass --repository."
        );
    }

    Ok(SyncSource {
        repository,
        git_ref: git(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|| "unknown".into()),
        commit_sha: git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "working-tree".into()),
        author: git(&["log", "-1", "--format=%ae"]).unwrap_or_default(),
    })
}

/// Turns any remote URL shape into `owner/repo`, so the same checkout reports
/// the same identity whether it was cloned over SSH or HTTPS.
fn detect_repository() -> Option<String> {
    let url = git(&["remote", "get-url", "origin"])?;
    let trimmed = url.trim_end_matches(".git");

    let path = if let Some(rest) = trimmed.split_once("://") {
        rest.1.split_once('/').map(|(_, p)| p).unwrap_or(rest.1)
    } else if let Some((_, rest)) = trimmed.split_once(':') {
        rest
    } else {
        trimmed
    };

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.len() {
        0 => None,
        1 => Some(segments[0].to_string()),
        n => Some(format!("{}/{}", segments[n - 2], segments[n - 1])),
    }
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

// ── output ───────────────────────────────────────────────────────────────────

fn print_text(response: &SyncResponse, dry_run: bool) {
    let summary = &response.plan.summary;

    if response.plan.changes.is_empty() {
        println!("No changes. Your file matches FlagDash.");
        return;
    }

    for change in &response.plan.changes {
        println!("{}", format_change_line(change));
        for line in diff_lines(change) {
            println!("      {line}");
        }
        if let Some(note) = &change.note {
            println!("      {note}");
        }
    }

    println!();
    println!(
        "{} change{}: {} to create, {} to update, {} to delete, {} drifted.",
        summary.total(),
        if summary.total() == 1 { "" } else { "s" },
        summary.create,
        summary.update,
        summary.delete,
        summary.drift
    );

    if dry_run {
        // Telling someone to run apply when apply is going to refuse wastes a
        // CI round trip and reads as a bug. Say what will actually happen.
        if summary.drift > 0 {
            println!(
                "\nSome of these were changed outside the repository, so `flagdash apply` will\n\
                 stop rather than revert them. Update the document to match, or apply with\n\
                 --accept-drift to overwrite them deliberately."
            );
        } else {
            println!("\nRun `flagdash apply` to make these changes.");
        }
    } else if let Some(results) = &response.results {
        println!(
            "\nApplied {} resource{}.",
            results.applied.len(),
            if results.applied.len() == 1 { "" } else { "s" }
        );
        for failure in &results.failed {
            println!("  failed: {failure}");
        }
    }
}

fn print_markdown(response: &SyncResponse, dry_run: bool) {
    let summary = &response.plan.summary;

    println!("## FlagDash {}", if dry_run { "plan" } else { "apply" });
    println!();

    if response.plan.changes.is_empty() {
        println!("No changes. The document matches FlagDash.");
        return;
    }

    println!("| | Resource | Environment | Change |");
    println!("|---|---|---|---|");

    for change in &response.plan.changes {
        let environment = change.environment.clone().unwrap_or_else(|| "—".into());
        let detail = diff_lines(change).join("<br>");
        let detail = if detail.is_empty() {
            change.action.clone()
        } else {
            detail
        };

        println!(
            "| {} | `{}` | {} | {} |",
            marker(&change.action),
            change.key,
            environment,
            detail
        );
    }

    println!();
    println!(
        "**{} change{}** — {} to create, {} to update, {} to delete, {} drifted.",
        summary.total(),
        if summary.total() == 1 { "" } else { "s" },
        summary.create,
        summary.update,
        summary.delete,
        summary.drift
    );

    if summary.drift > 0 {
        println!();
        println!(
            "> **Drift.** Resources above were changed outside this repository since the last \
             sync. Applying will not overwrite them until the document is updated to match, or \
             the apply is re-run with `--accept-drift`."
        );
    }
}

fn format_change_line(change: &Change) -> String {
    let environment = change
        .environment
        .as_ref()
        .map(|e| format!("  {e}"))
        .unwrap_or_default();

    format!("  {} {}{}", marker(&change.action), change.key, environment)
}

fn marker(action: &str) -> &'static str {
    match action {
        "create" => "+",
        "update" => "~",
        "delete" => "-",
        "drift" => "!",
        _ => "?",
    }
}

/// Renders a diff as `field: from → to`, which is the line a reviewer actually
/// reads on a pull request.
fn diff_lines(change: &Change) -> Vec<String> {
    let Some(fields) = change.diff.as_object() else {
        return Vec::new();
    };

    let mut keys: Vec<&String> = fields.keys().collect();
    keys.sort();

    keys.iter()
        .filter_map(|key| {
            let entry = fields.get(*key)?;
            let from = entry.get("from").map(render_value).unwrap_or_default();
            let to = entry.get("to").map(render_value).unwrap_or_default();

            if change.action == "create" {
                Some(format!("{key}: {to}"))
            } else {
                Some(format!("{key}: {from} → {to}"))
            }
        })
        .collect()
}

fn render_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "none".into(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => match map.get("value") {
            Some(inner) if map.len() == 1 => render_value(inner),
            _ => value.to_string(),
        },
        other => other.to_string(),
    }
}

fn report_failure(status: reqwest::StatusCode, body: &str) {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) else {
        eprintln!("FlagDash returned {status}: {body}");
        return;
    };

    let error = parsed.get("error").unwrap_or(&parsed);
    let message = error
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("the request was rejected");

    eprintln!("{message}");

    // Validation errors carry the dotted path to each problem, which is the
    // difference between "invalid document" and a line to go and fix.
    if let Some(errors) = error.get("errors").and_then(|e| e.as_array()) {
        for item in errors {
            let path = item.get("path").and_then(|p| p.as_str()).unwrap_or("");
            let detail = item.get("message").and_then(|m| m.as_str()).unwrap_or("");
            eprintln!("  {path}: {detail}");
        }
    }

    if let Some(changes) = error.get("changes").and_then(|c| c.as_array()) {
        for item in changes {
            let key = item.get("key").and_then(|k| k.as_str()).unwrap_or("");
            let note = item.get("note").and_then(|n| n.as_str()).unwrap_or("");
            eprintln!("  {key}: {note}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_ssh_and_https_remotes_to_the_same_identity() {
        // The same checkout must report one identity regardless of clone URL,
        // because that string is what claims ownership of a flag.
        for url in [
            "git@github.com:acme/web.git",
            "https://github.com/acme/web.git",
            "https://github.com/acme/web",
            "ssh://git@github.com/acme/web.git",
        ] {
            assert_eq!(
                repository_from_url(url).as_deref(),
                Some("acme/web"),
                "{url}"
            );
        }
    }

    #[test]
    fn renders_a_wrapped_value_as_the_bare_value() {
        let wrapped = serde_json::json!({"value": false});
        assert_eq!(render_value(&wrapped), "false");
    }

    #[test]
    fn renders_a_diff_as_an_arrow() {
        let change = Change {
            action: "update".into(),
            resource: "flag_environment".into(),
            key: "search-v2".into(),
            environment: Some("production".into()),
            diff: serde_json::json!({"rollout_percentage": {"from": 10, "to": 25}}),
            note: None,
        };

        assert_eq!(diff_lines(&change), vec!["rollout_percentage: 10 → 25"]);
    }

    #[test]
    fn a_create_shows_only_the_destination() {
        let change = Change {
            action: "create".into(),
            resource: "flag".into(),
            key: "new".into(),
            environment: None,
            diff: serde_json::json!({"name": {"from": null, "to": "New"}}),
            note: None,
        };

        assert_eq!(diff_lines(&change), vec!["name: New"]);
    }

    // Exercised through the same parsing `detect_repository` uses.
    fn repository_from_url(url: &str) -> Option<String> {
        let trimmed = url.trim_end_matches(".git");

        let path = if let Some(rest) = trimmed.split_once("://") {
            rest.1.split_once('/').map(|(_, p)| p).unwrap_or(rest.1)
        } else if let Some((_, rest)) = trimmed.split_once(':') {
            rest
        } else {
            trimmed
        };

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        match segments.len() {
            0 => None,
            1 => Some(segments[0].to_string()),
            n => Some(format!("{}/{}", segments[n - 2], segments[n - 1])),
        }
    }
}
