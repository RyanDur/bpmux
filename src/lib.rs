#![deny(unsafe_code)]

pub mod bridge;
pub mod mcp;
pub mod reload;
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
    /// Run in headless mode for agent testing (no terminal required)
    Test,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let headless = matches!(cli.command, Some(Command::Test));
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
    let session = Arc::new(session::Session::new(
        session_dir,
        cli.name.clone(),
        wake_tx,
    ));
    let messages = session.read_messages()?;
    let mut app = terminal::App::new(cli.name.clone(), session.id.clone(), messages);
    app.session_dir = Some(session.dir.clone());
    terminal::ui_state::UiState::load_and_restore(&mut app, &session);
    let server_session = Arc::clone(&session);
    let server_port = cli.mcp_port;
    let ct = CancellationToken::new();
    let server_ct = ct.clone();
    let screen_capture = Arc::new(std::sync::Mutex::new(String::new()));
    let (inject_tx, inject_rx) = tokio::sync::mpsc::channel::<crossterm::event::Event>(16);

    let rt = tokio::runtime::Runtime::new()?;
    let screen_for_server = Arc::clone(&screen_capture);
    let inject_for_server = inject_tx.clone();
    let result = rt.block_on(async {
        tokio::spawn(async move {
            if let Err(e) = server::run_server(
                server_session,
                server_port,
                server_ct,
                screen_for_server,
                inject_for_server,
            )
            .await
            {
                session::diag::log_diag(&format!("server: run error: {e}"));
            }
        });

        session::awareness::spawn(Arc::clone(&session), session.owner_name.clone());

        if headless {
            let backend = ratatui::backend::TestBackend::new(80, 24);
            let mut terminal = ratatui::Terminal::new(backend)?;
            eprintln!("bpmux: headless mode (80x24)");
            terminal::run_tui(
                &mut terminal,
                &mut app,
                &session,
                &mut wake_rx,
                Arc::clone(&screen_capture),
                inject_rx,
            )
            .await
        } else {
            crossterm::execute!(
                std::io::stdout(),
                crossterm::event::EnableMouseCapture,
                crossterm::event::EnableFocusChange,
                crossterm::event::EnableBracketedPaste,
            )?;
            let mut terminal = ratatui::init();
            let result = terminal::run_tui(
                &mut terminal,
                &mut app,
                &session,
                &mut wake_rx,
                Arc::clone(&screen_capture),
                inject_rx,
            )
            .await;
            ratatui::restore();
            crossterm::execute!(
                std::io::stdout(),
                crossterm::event::DisableMouseCapture,
                crossterm::event::DisableFocusChange,
                crossterm::event::DisableBracketedPaste,
            )?;
            result
        }
    });

    if app.should_reload {
        return reload::relaunch();
    }

    result.map_err(|e| anyhow::anyhow!("{e}"))
}
