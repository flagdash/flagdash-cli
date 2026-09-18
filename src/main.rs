// Allow dead code while features are being wired up
#![allow(dead_code)]

mod action;
mod api;
mod app;
mod components;
mod config;
mod event;
mod gitops;
mod secrets;
mod theme;
mod tui;
mod views;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "flagdash",
    about = "FlagDash TUI — Interactive terminal UI for feature flag management",
    version,
    author = "FlagDash <team@flagdash.io>"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Session token (overrides config file and env var)
    #[arg(long, env = "FLAGDASH_SESSION_TOKEN", global = true)]
    session_token: Option<String>,

    /// Management API key (legacy alias for --session-token)
    #[arg(long, env = "FLAGDASH_API_KEY", hide = true, global = true)]
    api_key: Option<String>,

    /// Base URL for FlagDash API
    #[arg(long, env = "FLAGDASH_BASE_URL", global = true)]
    base_url: Option<String>,

    /// Default project ID
    #[arg(long, env = "FLAGDASH_PROJECT_ID", global = true)]
    project_id: Option<String>,

    /// Default environment ID
    #[arg(long, env = "FLAGDASH_ENVIRONMENT_ID", global = true)]
    environment_id: Option<String>,
}

/// Non-interactive subcommands. Dispatched before the terminal is initialised,
/// so they never touch the TUI and are safe to run in CI.
#[derive(Subcommand, Debug)]
enum Command {
    /// Show what a GitOps document would change, without changing it
    ///
    /// Exits 0 when there is nothing to do and 2 when there are pending
    /// changes, so any CI can gate on the result without a plugin.
    Plan {
        /// Document to read (default: flagdash.yaml, or flagdash.d/)
        #[arg(long)]
        file: Option<std::path::PathBuf>,

        /// Output format: text, json or markdown
        #[arg(long, default_value = "text")]
        format: gitops::Format,

        /// Repository identity (default: detected from git)
        #[arg(long)]
        repository: Option<String>,

        /// Exit 0 even when there are pending changes
        #[arg(long)]
        exit_zero: bool,
    },

    /// Apply a GitOps document to FlagDash
    Apply {
        /// Document to read (default: flagdash.yaml, or flagdash.d/)
        #[arg(long)]
        file: Option<std::path::PathBuf>,

        /// Output format: text, json or markdown
        #[arg(long, default_value = "text")]
        format: gitops::Format,

        /// Delete resources this repository manages that the document no longer
        /// declares. Never touches resources it does not own.
        #[arg(long)]
        prune: bool,

        /// Overwrite resources that were changed outside the repository.
        /// Without this an apply stops rather than reverting them.
        #[arg(long)]
        accept_drift: bool,

        /// Repository identity (default: detected from git)
        #[arg(long)]
        repository: Option<String>,
    },

    /// GitOps helpers
    Gitops {
        #[command(subcommand)]
        command: GitopsCommand,
    },

    /// Manage encrypted secrets (metadata and lifecycle only)
    ///
    /// There is no command that prints a secret's value, and there never will
    /// be: a terminal is where credentials end up in scrollback, shell history
    /// and CI logs. Read one from your application with an `sk_` key holding
    /// `secrets:read`.
    Secrets {
        #[command(subcommand)]
        command: SecretsCommand,
    },
}

#[derive(Subcommand, Debug)]
enum SecretsCommand {
    /// List the secrets in the selected environment. Values are never shown.
    List,

    /// Fetch secret values for a CI step.
    ///
    /// Needs a project-scoped `sk_` key holding `secrets:read`, passed with
    /// --sdk-key or FLAGDASH_SDK_KEY — not your login token. That split is the
    /// point: a CI job gets exactly one capability, and an everyday `flagdash`
    /// on a developer's laptop cannot print a production credential.
    ///
    /// On GitHub Actions the values are registered with `::add-mask::` before
    /// they are printed, so they are redacted from the job log.
    ///
    ///   flagdash secrets fetch stripe-secret-key --format raw
    ///   flagdash secrets fetch db-password api-token --format env >> "$GITHUB_ENV"
    ///   flagdash secrets fetch gcp-credentials --format json --out /tmp/gcp.json
    Fetch {
        /// One or more secret keys
        #[arg(required = true)]
        keys: Vec<String>,

        /// Project-scoped SDK key with secrets:read
        #[arg(long, env = "FLAGDASH_SDK_KEY", hide_env_values = true)]
        sdk_key: Option<String>,

        /// Output format: raw (one key), env, or json
        #[arg(long, default_value = "raw")]
        format: secrets::OutputFormat,

        /// Write to this file (mode 0600) instead of stdout
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,

        /// Do not emit CI log-masking directives
        #[arg(long)]
        no_mask: bool,
    },

    /// Show one secret's metadata: format, status, current version.
    Show { key: String },

    /// Create a secret. The value is read from a file or stdin, never argv.
    Create {
        key: String,

        /// Display name (defaults to the key)
        #[arg(long)]
        name: Option<String>,

        /// Optional description
        #[arg(long, default_value = "")]
        description: String,

        /// Secret format: string or json
        #[arg(long, default_value = "string")]
        format: String,

        /// Read the value from this file
        #[arg(long, value_name = "PATH")]
        from_file: Option<std::path::PathBuf>,

        /// Read the value from stdin
        #[arg(long)]
        stdin: bool,
    },

    /// Replace a secret's value with a new encrypted version.
    Set {
        key: String,

        /// Read the new value from this file
        #[arg(long, value_name = "PATH")]
        from_file: Option<std::path::PathBuf>,

        /// Read the new value from stdin
        #[arg(long)]
        stdin: bool,
    },

    /// List a secret's version history (metadata only).
    Versions { key: String },

    /// Restore a historical version as a new encrypted version.
    Restore { key: String, version_id: String },

    /// Approve a pending change. Must be a different user than the author.
    Approve { key: String },

    /// Soft-delete a secret. Recoverable for seven days.
    Delete { key: String },

    /// Recover a soft-deleted secret within the recovery window.
    Recover { key: String },
}

#[derive(Subcommand, Debug)]
enum GitopsCommand {
    /// Print the project as a GitOps document
    ///
    /// The way to adopt GitOps on a project that already has flags: commit the
    /// output rather than hand-writing it.
    Export,
}

/// Create the log file with owner-only permissions (0600) on Unix so its
/// contents aren't readable by other local users. Falls back to a plain create
/// on other platforms. Returns None if creation fails (logging is best-effort).
#[cfg(unix)]
fn create_private_log_file(path: &std::path::Path) -> Option<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .ok()
}

#[cfg(not(unix))]
fn create_private_log_file(path: &std::path::Path) -> Option<std::fs::File> {
    std::fs::File::create(path).ok()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args
    let cli = Cli::parse();

    // Install panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = tui::restore();
        original_hook(panic_info);
    }));

    // Initialize tracing (logs to file, not stdout).
    // Default to INFO to avoid capturing verbose dependency (reqwest/hyper) logs
    // into a file; opt into DEBUG only when FLAGDASH_DEBUG is set. The log file
    // is created owner-only (0600) on Unix so it isn't world-readable.
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("flagdash");
    std::fs::create_dir_all(&log_dir).ok();
    let log_file = create_private_log_file(&log_dir.join("flagdash.log"));
    if let Some(file) = log_file {
        let level = if std::env::var_os("FLAGDASH_DEBUG").is_some() {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        };
        tracing_subscriber::fmt()
            .with_writer(file)
            .with_ansi(false)
            .with_max_level(level)
            .init();
    }

    // --session-token takes priority, --api-key is a fallback
    let token = cli.session_token.or(cli.api_key);

    // Load config with priority: CLI args > env vars > config file
    let app_config = config::AppConfig::load(
        token.as_deref(),
        cli.base_url.as_deref(),
        cli.project_id.as_deref(),
        cli.environment_id.as_deref(),
    )?;

    // Subcommands run headless and exit; only the bare invocation opens the TUI.
    if let Some(command) = cli.command {
        match command {
            Command::Plan {
                file,
                format,
                repository,
                exit_zero,
            } => {
                let options = gitops::Options {
                    file,
                    format,
                    prune: false,
                    accept_drift: false,
                    repository,
                    exit_zero,
                };
                let code = gitops::plan(&app_config, &options).await?;
                std::process::exit(code);
            }

            Command::Apply {
                file,
                format,
                prune,
                accept_drift,
                repository,
            } => {
                let options = gitops::Options {
                    file,
                    format,
                    prune,
                    accept_drift,
                    repository,
                    exit_zero: true,
                };
                let code = gitops::apply(&app_config, &options).await?;
                std::process::exit(code);
            }

            Command::Gitops {
                command: GitopsCommand::Export,
            } => {
                gitops::export(&app_config).await?;
                return Ok(());
            }

            Command::Secrets { command } => {
                match command {
                    SecretsCommand::List => secrets::list(&app_config).await?,
                    SecretsCommand::Fetch {
                        keys,
                        sdk_key,
                        format,
                        out,
                        no_mask,
                    } => {
                        let options = secrets::FetchOptions {
                            keys,
                            sdk_key,
                            format,
                            out,
                            no_mask,
                        };
                        secrets::fetch(&app_config, &options).await?
                    }
                    SecretsCommand::Show { key } => secrets::show(&app_config, &key).await?,
                    SecretsCommand::Create {
                        key,
                        name,
                        description,
                        format,
                        from_file,
                        stdin,
                    } => {
                        let source = secrets::ValueSource {
                            file: from_file,
                            stdin,
                        };
                        secrets::create(
                            &app_config,
                            &key,
                            name.as_deref(),
                            &description,
                            &format,
                            &source,
                        )
                        .await?
                    }
                    SecretsCommand::Set {
                        key,
                        from_file,
                        stdin,
                    } => {
                        let source = secrets::ValueSource {
                            file: from_file,
                            stdin,
                        };
                        secrets::set(&app_config, &key, &source).await?
                    }
                    SecretsCommand::Versions { key } => {
                        secrets::versions(&app_config, &key).await?
                    }
                    SecretsCommand::Restore { key, version_id } => {
                        secrets::restore(&app_config, &key, &version_id).await?
                    }
                    SecretsCommand::Approve { key } => secrets::approve(&app_config, &key).await?,
                    SecretsCommand::Delete { key } => secrets::delete(&app_config, &key).await?,
                    SecretsCommand::Recover { key } => secrets::recover(&app_config, &key).await?,
                }
                return Ok(());
            }
        }
    }

    // Initialize terminal
    let mut terminal = tui::init()?;

    // Create app
    let mut app = app::App::new(app_config);
    let mut events = event::EventHandler::new(250); // 4 ticks/sec

    // Main event loop
    while app.running {
        // Draw
        terminal.draw(|frame| app.render(frame))?;

        // Handle events
        tokio::select! {
            event = events.next() => {
                if let Ok(event) = event {
                    app.handle_event(&event)?;
                }
            }
            Some(action) = app.action_rx.recv() => {
                app.process_action(action);
            }
        }
    }

    // Restore terminal
    tui::restore()?;

    Ok(())
}
