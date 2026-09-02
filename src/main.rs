// Allow dead code while features are being wired up
#![allow(dead_code)]

mod action;
mod api;
mod app;
mod components;
mod config;
mod event;
mod gitops;
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
