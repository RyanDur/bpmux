use std::sync::Arc;

use clap::Parser;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::{bridge, cli, hub, location, server, transport};

mod resolve;
mod room;

#[cfg(test)]
pub(crate) use resolve::{or_register_user_id_from_pin_or_dir, registered_user_id_from_pin_or_dir};
pub(crate) use resolve::{
    resolve_bpmux_port, resolve_or_register_user_id, resolve_registered_user_id, resolve_role,
};

use resolve::resolve_data_base;
use room::{run_room_kill, run_room_list, run_room_read, run_room_spawn};

#[derive(Parser)]
#[command(
    name = "bpmux",
    version,
    about = "P2P group chat for AI pair programming",
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    #[arg(long, default_value = "dev", help = "Your display name")]
    name: String,

    #[arg(
        long,
        default_value_t = transport::local_url::DEFAULT_API_PORT,
        help = "MCP server port"
    )]
    mcp_port: u16,

    #[arg(long, help = "Data base directory (defaults to ~/.bpmux/data)")]
    data_base: Option<String>,

    #[command(subcommand)]
    command: Option<cli::Command>,
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
    let base = resolve_data_base(&cli.data_base)?;
    let data_dir = match location::resume_or_create_data_dir(&base) {
        Ok(dir) => dir,
        Err(_) => return Ok(()),
    };
    let lessons = crate::memory::recall::surface_for_role(&data_dir, &role);
    print!(
        "{}",
        crate::memory::recall::format_for_surface(&role, &lessons)
    );
    cli::emit_recall_activity(&role, lessons.len());
    Ok(())
}

fn run_server_only(cli: &Cli) -> anyhow::Result<()> {
    let workflow = workflow::load_workflow_from_process_env()
        .map_err(|e| anyhow::anyhow!("refusing to start on a broken workflow config: {e}"))?;
    workflow::install_workflow(workflow);
    let base = resolve_data_base(&cli.data_base)?;
    std::fs::create_dir_all(&base)?;
    let data_dir = location::resume_or_create_data_dir(&base)?;
    let (wake_tx, _wake_rx) =
        broadcast::channel::<message::Message>(hub::BROADCAST_CHANNEL_CAPACITY);
    let hub = Arc::new(hub::Hub::new(data_dir, cli.name.clone(), wake_tx));
    let port = cli.mcp_port;
    let ct = CancellationToken::new();
    let restart_signal = transport::self_update::RestartSignal::new();
    let rt = tokio::runtime::Runtime::new()?;
    let server_result = rt.block_on({
        let ct = ct.clone();
        let restart_signal = restart_signal.clone();
        async move {
            crate::awareness::spawn(Arc::clone(&hub), hub.owner_name.clone());
            transport::self_update::spawn_binary_watcher(ct.clone(), restart_signal);
            spawn_shutdown_signal_listener(ct.clone());
            server::run_server(hub, port, ct)
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
                diag::log_diag("server: SIGTERM received; cancelling CT");
            }
            _ = sigint => {
                diag::log_diag("server: SIGINT received; cancelling CT");
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

    if let Some(command) = &cli.command
        && let Some(result) = cli::dispatch(command)
    {
        return result;
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
