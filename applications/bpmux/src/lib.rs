#![deny(unsafe_code)]

pub mod board;
pub mod bridge;
pub mod ci;
#[cfg(test)]
mod ci_tests;
mod cli;
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

use clap::Parser;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(
    name = "bpmux",
    version,
    about = "P2P group chat for AI pair programming",
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
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
    command: Option<cli::Command>,
}

fn run_room_spawn(role: &str, agent: &str, headless: bool) -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let pid = if headless {
        room::spawn_captured(agent, &[], &storage.output_path(role))?
    } else {
        room::spawn_pty(agent, &[])?
    };
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

fn run_room_read(role: &str, lines: Option<usize>) -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let raw = <room::FileRoomStorage as room::RoomStore>::read(&storage, role, lines)?;
    print!("{}", room::strip_ansi(&raw));
    Ok(())
}

fn run_room_kill(role: &str) -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let entry = <room::FileRoomStorage as room::RoomStore>::get(&storage, role)?;
    room::terminate(entry.pid)?;
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
    cli::emit_recall_activity(&role, lessons.len());
    Ok(())
}

pub(crate) fn resolve_bpmux_port() -> u16 {
    std::env::var("BPMUX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(transport::local_url::DEFAULT_API_PORT)
}

pub(crate) fn resolve_role(role_arg: &Option<String>) -> String {
    role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string())
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

    if let Some(cli::Command::Bridge {
        port,
        name,
        fake_tracker,
        board_fixture,
    }) = cli.command
    {
        return bridge::run_bridge(port, &name, fake_tracker, board_fixture);
    }

    if let Some(cli::Command::Recall { role }) = &cli.command {
        return run_recall(&cli, role);
    }

    if let Some(cli::Command::EmitPrecommitPassed { sha, role }) = &cli.command {
        return cli::run_emit_precommit_passed(sha, role);
    }

    if let Some(cli::Command::WaitForEvent { after, timeout_ms }) = &cli.command {
        return cli::run_wait_for_event(after, *timeout_ms);
    }

    if let Some(cli::Command::Send {
        text,
        to,
        addressee,
    }) = &cli.command
    {
        return cli::run_send(text, to, addressee);
    }

    if let Some(cli::Command::Read { limit, full }) = &cli.command {
        return cli::run_read(*limit, *full);
    }

    if let Some(cli::Command::Help { json }) = &cli.command {
        return cli::run_help(*json);
    }

    if let Some(cli::Command::Ready { json, full }) = &cli.command {
        return cli::run_ready(*json, *full);
    }

    if let Some(cli::Command::Story { number, json, full }) = &cli.command {
        return cli::run_story(*number, *json, *full);
    }

    if let Some(cli::Command::Board {
        state,
        limit,
        json,
        full,
    }) = &cli.command
    {
        return cli::run_board(state, *limit, *json, *full);
    }

    if let Some(cli::Command::Pass { number, acceptor }) = &cli.command {
        return cli::run_pass(*number, acceptor);
    }

    if let Some(cli::Command::Start {
        number,
        paraphrase,
        falsifier,
    }) = &cli.command
    {
        return cli::run_start(*number, paraphrase, falsifier);
    }

    if let Some(cli::Command::Deliver {
        number,
        observation,
    }) = &cli.command
    {
        return cli::run_deliver(*number, observation);
    }

    if let Some(cli::Command::Fail { number, reason }) = &cli.command {
        return cli::run_fail(*number, reason);
    }

    if let Some(cli::Command::Reject { number, reason }) = &cli.command {
        return cli::run_reject(*number, reason);
    }

    if let Some(cli::Command::Verdict {
        number,
        verdict,
        grounding,
    }) = &cli.command
    {
        return cli::run_verdict(*number, verdict, grounding);
    }

    if let Some(cli::Command::Accept {
        number,
        would_file_bug,
        force,
        bypass_reason,
    }) = &cli.command
    {
        return cli::run_accept(*number, *would_file_bug, *force, bypass_reason);
    }

    if let Some(cli::Command::Move {
        number,
        target_state,
        rationale,
        position,
    }) = &cli.command
    {
        return cli::run_move(*number, target_state, rationale, *position);
    }

    if let Some(cli::Command::Label {
        number,
        label,
        operation,
    }) = &cli.command
    {
        return cli::run_label(*number, label, operation);
    }

    if let Some(cli::Command::Create {
        title,
        user_job,
        observation_falsifier,
        body,
        parent_epic,
        acceptance_test_path,
    }) = &cli.command
    {
        return cli::run_create(
            title,
            user_job,
            observation_falsifier,
            body,
            *parent_epic,
            acceptance_test_path,
        );
    }

    if let Some(cli::Command::Edit {
        number,
        title,
        body,
    }) = &cli.command
    {
        return cli::run_edit(*number, title, body);
    }

    if let Some(cli::Command::Comment { number, body }) = &cli.command {
        return cli::run_comment(*number, body);
    }

    if let Some(cli::Command::Close {
        number,
        reason,
        disposition,
    }) = &cli.command
    {
        return cli::run_close(*number, reason, disposition);
    }

    if let Some(cli::Command::Iteration { action }) = &cli.command {
        return match action {
            cli::IterationAction::Create {
                theme,
                position_after,
            } => cli::run_iteration_create(theme, *position_after),
            cli::IterationAction::Designate { number } => cli::run_iteration_designate(*number),
        };
    }

    if let Some(cli::Command::Room { action }) = &cli.command {
        return match action {
            cli::RoomAction::Spawn {
                role,
                agent,
                headless,
            } => run_room_spawn(role, agent, *headless),
            cli::RoomAction::List {} => run_room_list(),
            cli::RoomAction::Read { role, lines } => run_room_read(role, *lines),
            cli::RoomAction::Kill { role } => run_room_kill(role),
        };
    }

    run_server_only(&cli)
}
