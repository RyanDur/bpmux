#![deny(unsafe_code)]

pub mod bridge;
pub mod mcp;
pub mod server;
pub mod session;
pub mod terminal;
pub mod transport;

use std::sync::Arc;

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

        /// Base identity name for this bridge (a unique suffix is appended)
        #[arg(long, default_value = "claude-code")]
        name: String,
    },
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(Command::Bridge { port, name }) = cli.command {
        return bridge::run_bridge(port, &name);
    }

    let base = match &cli.session_base {
        Some(path) => std::path::PathBuf::from(path),
        None => match std::env::var("BPMUX_SESSION_BASE") {
            Ok(path) => std::path::PathBuf::from(path),
            Err(_) => session::default_sessions_base()?,
        },
    };
    std::fs::create_dir_all(&base)?;
    let session_dir = session::resume_or_create_session(&base)?;

    let (wake_tx, mut wake_rx) = broadcast::channel::<session::Message>(256);
    let session = Arc::new(session::Session::new(session_dir, wake_tx));
    let messages = session.read_messages()?;
    let pins = session.read_pins()?;
    let is_first_launch = messages.is_empty();
    let mut app = terminal::App::new(cli.name.clone(), session.id.clone(), messages);
    app.session_dir = Some(session.dir.clone());
    app.pins = pins;
    // Inject welcome or resume message
    if is_first_launch {
        app.messages.push(session::Message {
            from: "bpmux".to_string(),
            text: format!(
                "Welcome to bpmux — the shared message bus for AI pair programming.\n\
                 \n\
                 You are {name}. Type a message and press Enter to broadcast.\n\
                 Agents connect via WebSocket on port {port}.\n\
                 \n\
                 Quick start:\n\
                 \x20 /to dev hello  — send to a specific participant\n\
                 \x20 Ctrl+B         — toggle channel sidebar\n\
                 \x20 ?              — show help overlay\n\
                 \x20 /who           — list connected participants\n\
                 \n\
                 Session: {sid}",
                name = cli.name,
                port = cli.mcp_port,
                sid = session.id,
            ),
            ts: chrono::Utc::now(),
            message_type: None,
            to: None,
        });
    } else {
        let msg_count = app.messages.len();
        app.messages.push(session::Message {
            from: "bpmux".to_string(),
            text: format!(
                "Resumed session {sid} — {msg_count} message{s} in log. Type /help for commands.",
                sid = session.id,
                s = if msg_count == 1 { "" } else { "s" },
            ),
            ts: chrono::Utc::now(),
            message_type: None,
            to: None,
        });
    }

    let server_session = Arc::clone(&session);
    let server_name = cli.name.clone();
    let server_port = cli.mcp_port;
    let ct = CancellationToken::new();
    let server_ct = ct.clone();
    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Runtime::new() else {
            eprintln!("Failed to create server tokio runtime");
            return;
        };
        rt.block_on(async move {
            if let Err(e) =
                server::run_server(server_session, server_name, server_port, server_ct).await
            {
                eprintln!("Server error: {e}");
            }
        });
    });

    // Enable mouse capture and focus tracking
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableFocusChange,
    )?;
    let mut terminal = ratatui::init();
    let result = terminal::run_tui(&mut terminal, &mut app, &session, &mut wake_rx);
    ratatui::restore();
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableFocusChange,
    )?;

    result.map_err(|e| anyhow::anyhow!("{e}"))
}
