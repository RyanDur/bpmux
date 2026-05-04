#![deny(unsafe_code)]

pub mod board;
pub mod bridge;
pub mod ci;
#[cfg(test)]
mod ci_tests;
pub mod github;
pub mod mcp;
pub mod reload;
pub mod roadmap;
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

        /// Use fake story tracker (no gh calls). Test infrastructure only.
        #[arg(long)]
        fake_tracker: bool,

        /// Path to a JSON fixture file for list_stories. Test infrastructure only.
        #[arg(long)]
        board_fixture: Option<String>,
    },
    /// Run in headless mode for agent testing (no terminal required)
    Test,
    /// Run the WS server + session without a TUI (Electron shell spawns this).
    Server,
    /// Print an agent's recent lessons to stdout for session-start auto-surface (#157)
    Recall {
        /// Role to surface lessons for (e.g. dev, pm, designer).
        /// Defaults to BPMUX_ROLE env var if unset.
        #[arg(long)]
        role: Option<String>,
    },
    /// Emit a pre-commit-gates-passed activity broadcast (#424). Invoked
    /// by the git post-commit hook with the SHA of the just-created
    /// commit. Fires only on gate-passed state because the commit
    /// wouldn't exist if pre-commit had failed.
    EmitPrecommitPassed {
        /// SHA of the commit whose pre-commit gates just passed.
        #[arg(long)]
        sha: String,
        /// Role to attribute the broadcast to. Defaults to BPMUX_ROLE
        /// env var; falls back to "dev".
        #[arg(long)]
        role: Option<String>,
    },
}

fn resolve_session_base(cli_base: &Option<String>) -> anyhow::Result<std::path::PathBuf> {
    match cli_base {
        Some(path) => Ok(std::path::PathBuf::from(path)),
        None => match std::env::var("BPMUX_SESSION_BASE") {
            Ok(path) => Ok(std::path::PathBuf::from(path)),
            Err(_) => Ok(session::default_sessions_base()?),
        },
    }
}

fn run_recall(cli: &Cli, role_arg: &Option<String>) -> anyhow::Result<()> {
    let role = role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .unwrap_or_default();
    if role.is_empty() {
        eprintln!("bpmux recall: no role specified (pass --role or set BPMUX_ROLE)");
        return Ok(());
    }
    let base = resolve_session_base(&cli.session_base)?;
    let session_dir = match session::resume_or_create_session(&base) {
        Ok(dir) => dir,
        Err(_) => return Ok(()),
    };
    let lessons = session::recall::surface_for_role(&session_dir, &role);
    print!("{}", session::recall::format_for_surface(&role, &lessons));
    // Emit a user-visible activity line into the bpmux channel
    // so Ryan can observe that recall fired without asking the agent
    // to quote its private system-reminder back. Best-effort — if the
    // bpmux TUI isn't running, the POST fails silently so the hook
    // never blocks session start.
    emit_recall_activity(&role, lessons.len());
    Ok(())
}

fn resolve_bpmux_port() -> u16 {
    std::env::var("BPMUX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4117)
}

fn emit_recall_activity(role: &str, count: usize) {
    let body = session::recall::build_recall_activity_payload(role, count);
    post_activity(&body);
}

fn resolve_role(role_arg: &Option<String>) -> String {
    role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string())
}

fn run_emit_precommit_passed(sha: &str, role_arg: &Option<String>) -> anyhow::Result<()> {
    let role = resolve_role(role_arg);
    let body = session::precommit::build_precommit_activity_payload(&role, sha);
    post_activity(&body);
    Ok(())
}

fn post_activity(body: &serde_json::Value) {
    let port = resolve_bpmux_port();
    let url = format!("http://127.0.0.1:{port}/api/messages");
    // Fire the POST from a one-shot tokio runtime. reqwest is async
    // and this entry point is sync; the runtime is cheap and local.
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build();
        let Ok(client) = client else { return };
        let _ = client.post(&url).json(&body).send().await;
    });
}

fn run_server_only(cli: &Cli) -> anyhow::Result<()> {
    let base = resolve_session_base(&cli.session_base)?;
    std::fs::create_dir_all(&base)?;
    let session_dir = session::resume_or_create_session(&base)?;
    let (wake_tx, _wake_rx) =
        broadcast::channel::<session::Message>(session::BROADCAST_CHANNEL_CAPACITY);
    let session = Arc::new(session::Session::new(
        session_dir,
        cli.name.clone(),
        wake_tx,
    ));
    let port = cli.mcp_port;
    let ct = CancellationToken::new();
    let screen_capture = Arc::new(std::sync::Mutex::new(String::new()));
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        session::awareness::spawn(Arc::clone(&session), session.owner_name.clone());
        server::run_server(session, port, ct, screen_capture)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    })
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let headless = matches!(cli.command, Some(Command::Test));
    if let Some(Command::Bridge {
        port,
        name,
        fake_tracker,
        board_fixture,
    }) = cli.command
    {
        return bridge::run_bridge(port, &name, fake_tracker, board_fixture);
    }

    if let Some(Command::Recall { role }) = &cli.command {
        return run_recall(&cli, role);
    }

    if let Some(Command::EmitPrecommitPassed { sha, role }) = &cli.command {
        return run_emit_precommit_passed(sha, role);
    }

    if matches!(cli.command, Some(Command::Server)) {
        return run_server_only(&cli);
    }

    let base = resolve_session_base(&cli.session_base)?;
    std::fs::create_dir_all(&base)?;
    let session_dir = session::resume_or_create_session(&base)?;

    let (wake_tx, mut wake_rx) =
        broadcast::channel::<session::Message>(session::BROADCAST_CHANNEL_CAPACITY);
    let session = Arc::new(session::Session::new(
        session_dir,
        cli.name.clone(),
        wake_tx,
    ));
    let messages = session.read_messages()?;
    let mut app = terminal::App::new(cli.name.clone(), session.id.clone(), messages);
    app.identity.dir = Some(session.dir.clone());
    terminal::ui_state::UiState::load_and_restore(&mut app, &session);
    let server_session = Arc::clone(&session);
    let server_port = cli.mcp_port;
    let ct = CancellationToken::new();
    let server_ct = ct.clone();
    let screen_capture = Arc::new(std::sync::Mutex::new(String::new()));
    let (_inject_tx, inject_rx) =
        tokio::sync::mpsc::channel::<crate::terminal::event::TerminalInjectEvent>(16);

    let rt = tokio::runtime::Runtime::new()?;
    let screen_for_server = Arc::clone(&screen_capture);
    let result = rt.block_on(async {
        tokio::spawn(async move {
            if let Err(e) =
                server::run_server(server_session, server_port, server_ct, screen_for_server).await
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
                true,
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
                false,
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

    if app.lifecycle.should_reload {
        return reload::relaunch();
    }

    result.map_err(|e| anyhow::anyhow!("{e}"))
}
