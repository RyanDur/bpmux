#![deny(unsafe_code)]

use clap::Parser;
use cli::Cli;

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(cli::Command::Bridge {
        port,
        name,
        fake_tracker,
        board_fixture,
    }) = cli.command
    {
        return mcp::run(port, &name, fake_tracker, board_fixture);
    }

    if let Some(cli::Command::Recall { role }) = &cli.command {
        return cli::run_recall(&cli, role);
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
            } => cli::run_room_spawn(role, agent, *headless),
            cli::RoomAction::List {} => cli::run_room_list(),
            cli::RoomAction::Read { role, lines } => cli::run_room_read(role, *lines),
            cli::RoomAction::Kill { role } => cli::run_room_kill(role),
        };
    }

    let base = cli::resolve_data_base(&cli.data_base)?;
    server::run(cli.name.clone(), base, cli.mcp_port)
}
