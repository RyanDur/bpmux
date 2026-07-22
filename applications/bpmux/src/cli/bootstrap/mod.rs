use clap::Parser;

use crate::{bridge, cli, location};

mod resolve;
mod room;

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
        default_value_t = local_url::DEFAULT_API_PORT,
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
    let lessons = memory::recall::surface_for_role(&data_dir, &role);
    print!("{}", memory::recall::format_for_surface(&role, &lessons));
    cli::emit_recall_activity(&role, lessons.len());
    Ok(())
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
        let user_id = std::env::current_dir()
            .ok()
            .and_then(|cwd| resolve_registered_user_id(&cwd));
        return bridge::run_bridge(port, &name, user_id, fake_tracker, board_fixture);
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

    let base = resolve_data_base(&cli.data_base)?;
    server::run(cli.name.clone(), base, cli.mcp_port)
}
