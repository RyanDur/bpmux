#![deny(unsafe_code)]

pub mod mcp;
pub mod session;
pub mod terminal;

use clap::Parser;
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
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

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

    // Wake channel — MCP server sends wake signals when messages arrive
    let (wake_tx, mut wake_rx) = broadcast::channel::<()>(16);

    // Start MCP/HTTP server on a dedicated thread with its own tokio runtime
    let mcp_session_dir = session_dir.clone();
    let mcp_name = cli.name.clone();
    let mcp_port = cli.mcp_port;
    let ct = CancellationToken::new();
    let server_ct = ct.clone();
    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Runtime::new() else {
            eprintln!("Failed to create server tokio runtime");
            return;
        };
        rt.block_on(async move {
            if let Err(e) =
                mcp::run_mcp_server(mcp_session_dir, mcp_name, wake_tx, mcp_port, server_ct).await
            {
                eprintln!("MCP server error: {e}");
            }
        });
    });

    // Run TUI (blocking, on the main thread)
    let mut terminal = ratatui::init();
    let result = terminal::run_tui(&mut terminal, &mut app, &session_dir, &mut wake_rx);
    ratatui::restore();

    result.map_err(|e| anyhow::anyhow!("{e}"))
}
