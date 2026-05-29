#![deny(unsafe_code)]

pub mod board;
pub mod bridge;
pub mod ci;
#[cfg(test)]
mod ci_tests;
pub mod github;
pub mod mcp;
pub mod orchestrator;
pub mod reload;
pub mod roadmap;
pub mod room;
pub mod server;
pub mod session;
pub mod transport;
pub mod workflow;

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
    #[arg(long, default_value_t = transport::local_url::DEFAULT_API_PORT)]
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
        #[arg(long, default_value_t = transport::local_url::DEFAULT_API_PORT)]
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
    Room {
        #[command(subcommand)]
        action: RoomAction,
    },
}

#[derive(Subcommand)]
enum RoomAction {
    Spawn {
        role: String,
        #[arg(long, default_value = "claude-code")]
        agent: String,
        #[arg(long)]
        headless: bool,
    },
    List {},
}

fn run_room_spawn(role: &str, agent: &str, _headless: bool) -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let pid = room::spawn_pty(agent, &[])?;
    let entry = room::Room {
        role: role.to_string(),
        agent: agent.to_string(),
        pid,
        created_at: chrono::Utc::now(),
    };
    <room::FileRoomStorage as room::RoomStore>::save(&storage, &entry)?;
    Ok(())
}

fn run_room_list() -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let mut rooms = <room::FileRoomStorage as room::RoomStore>::list(&storage)?;
    rooms.sort_by(|a, b| a.role.cmp(&b.role));
    for room in &rooms {
        println!("{}\t{}\t{}", room.role, room.agent, room.pid);
    }
    Ok(())
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
    emit_recall_activity(&role, lessons.len());
    Ok(())
}

fn resolve_bpmux_port() -> u16 {
    std::env::var("BPMUX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(transport::local_url::DEFAULT_API_PORT)
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
    let url = transport::local_url::api_url(port, "/api/messages");
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
    let restart_signal = transport::self_update::RestartSignal::new();
    let screen_capture = Arc::new(std::sync::Mutex::new(String::new()));
    let rt = tokio::runtime::Runtime::new()?;
    let server_result = rt.block_on({
        let ct = ct.clone();
        let restart_signal = restart_signal.clone();
        async move {
            session::awareness::spawn(Arc::clone(&session), session.owner_name.clone());
            transport::self_update::spawn_binary_watcher(ct.clone(), restart_signal);
            spawn_shutdown_signal_listener(ct.clone());
            server::run_server(session, port, ct, screen_capture)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        }
    });
    if restart_signal.was_fired() {
        eprintln!(
            "[bpmux] binary changed on disk; exiting with code {} so the supervisor respawns",
            transport::self_update::RESTART_EXIT_CODE
        );
        std::process::exit(transport::self_update::RESTART_EXIT_CODE);
    }
    server_result
}

fn spawn_shutdown_signal_listener(ct: CancellationToken) {
    tokio::spawn(async move {
        let sigterm = async {
            #[cfg(unix)]
            {
                if let Ok(mut stream) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    stream.recv().await;
                }
            }
            #[cfg(not(unix))]
            std::future::pending::<()>().await;
        };
        let sigint = tokio::signal::ctrl_c();
        tokio::select! {
            _ = sigterm => {
                crate::session::diag::log_diag("server: SIGTERM received; cancelling CT");
            }
            _ = sigint => {
                crate::session::diag::log_diag("server: SIGINT received; cancelling CT");
            }
        }
        ct.cancel();
    });
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

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

    if let Some(Command::Room { action }) = &cli.command {
        return match action {
            RoomAction::Spawn {
                role,
                agent,
                headless,
            } => run_room_spawn(role, agent, *headless),
            RoomAction::List {} => run_room_list(),
        };
    }

    run_server_only(&cli)
}
