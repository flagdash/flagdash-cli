// Allow dead code while features are being wired up
#![allow(dead_code)]

mod action;
mod api;
mod app;
mod components;
mod config;
mod event;
mod theme;
mod tui;
mod views;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "flagdash",
    about = "FlagDash TUI — Interactive terminal UI for feature flag management",
    version,
    author = "FlagDash <team@flagdash.io>"
)]
struct Cli {
    /// Session token (overrides config file and env var)
    #[arg(long, env = "FLAGDASH_SESSION_TOKEN")]
    session_token: Option<String>,

    /// Management API key (legacy alias for --session-token)
    #[arg(long, env = "FLAGDASH_API_KEY", hide = true)]
    api_key: Option<String>,

    /// Base URL for FlagDash API
    #[arg(long, env = "FLAGDASH_BASE_URL")]
    base_url: Option<String>,

    /// Default project ID
    #[arg(long, env = "FLAGDASH_PROJECT_ID")]
    project_id: Option<String>,

    /// Default environment ID
    #[arg(long, env = "FLAGDASH_ENVIRONMENT_ID")]
    environment_id: Option<String>,
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

    // Initialize tracing (logs to file, not stdout)
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("flagdash");
    std::fs::create_dir_all(&log_dir).ok();
    let log_file = std::fs::File::create(log_dir.join("flagdash.log")).ok();
    if let Some(file) = log_file {
        tracing_subscriber::fmt()
            .with_writer(file)
            .with_ansi(false)
            .with_max_level(tracing::Level::DEBUG)
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
