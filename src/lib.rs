#![deny(unsafe_code)]

pub mod bridge;
pub mod mcp;
pub mod server;
pub mod session;
pub mod terminal;
pub mod transport;

use clap::{Parser, Subcommand};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

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

    /// MCP server port
    #[arg(long, default_value = "4117")]
    mcp_port: u16,

    /// Session base directory (defaults to ~/.bpmux/sessions)
    #[arg(long)]
    session_base: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run as a channel bridge for Claude Code integration
    Bridge {
        /// bpmux server port to connect to
        #[arg(long, default_value = "4117")]
        port: u16,
    },
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Dispatch subcommands
    if let Some(Command::Bridge { port }) = cli.command {
        return bridge::run_bridge(port);
    }

    // Default: run the TUI + server

    // Resume existing session or create new one
    // Priority: CLI flag > env var > default
    let base = match &cli.session_base {
        Some(path) => std::path::PathBuf::from(path),
        None => match std::env::var("BPMUX_SESSION_BASE") {
            Ok(path) => std::path::PathBuf::from(path),
            Err(_) => session::default_sessions_base()?,
        },
    };
    std::fs::create_dir_all(&base)?;
    let session_dir = session::resume_or_create_session(&base)?;
    let session_id = session_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Load existing messages
    let messages = session::read_messages(&session_dir)?;

    // Create app state
    let mut app = terminal::App::new(cli.name.clone(), session_id, messages);

    // Wake channel -- server sends wake signals when messages arrive
    let (wake_tx, mut wake_rx) = broadcast::channel::<session::Message>(64);

    // Start server on a dedicated thread with its own tokio runtime
    let server_session_dir = session_dir.clone();
    let server_name = cli.name.clone();
    let server_port = cli.mcp_port;
    let server_wake_tx = wake_tx.clone();
    let ct = CancellationToken::new();
    let server_ct = ct.clone();
    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Runtime::new() else {
            eprintln!("Failed to create server tokio runtime");
            return;
        };
        rt.block_on(async move {
            if let Err(e) = server::run_server(
                server_session_dir,
                server_name,
                server_wake_tx,
                server_port,
                server_ct,
            )
            .await
            {
                eprintln!("Server error: {e}");
            }
        });
    });

    // Run TUI (blocking, on the main thread)
    let mut terminal = ratatui::init();
    let result = terminal::run_tui(
        &mut terminal,
        &mut app,
        &session_dir,
        &mut wake_rx,
        &wake_tx,
    );
    ratatui::restore();

    result.map_err(|e| anyhow::anyhow!("{e}"))
}
