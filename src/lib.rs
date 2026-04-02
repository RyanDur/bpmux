#![deny(unsafe_code)]

pub mod session;
pub mod terminal;

use clap::Parser;
use tokio::sync::broadcast;

#[derive(Parser)]
#[command(
    name = "bpmux",
    version,
    about = "P2P group chat for AI pair programming"
)]
struct Cli {
    /// Your name in the session
    #[arg(long, default_value = "dev")]
    name: String,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Create session
    let session_dir = session::create_session_dir()?;
    let session_id = session_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Load existing messages
    let messages = session::read_messages(&session_dir)?;

    // Create app state
    let mut app = terminal::App::new(cli.name, session_id, messages);

    // Wake channel (unused in Story 1, wired for Story 2)
    let (_wake_tx, mut wake_rx) = broadcast::channel::<()>(16);

    // Run TUI
    let mut terminal = ratatui::init();
    let result = terminal::run_tui(&mut terminal, &mut app, &session_dir, &mut wake_rx);
    ratatui::restore();

    result.map_err(|e| anyhow::anyhow!("{e}"))
}
