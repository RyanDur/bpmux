use clap::Parser;

mod resolve;
mod room;

pub use resolve::{
    resolve_bpmux_port, resolve_or_register_user_id, resolve_registered_user_id, resolve_role,
};

pub use resolve::resolve_data_base;
pub use room::{run_room_kill, run_room_list, run_room_read, run_room_spawn};

#[derive(Parser)]
#[command(
    name = "bpmux",
    version,
    about = "P2P group chat for AI pair programming",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(long, default_value = "dev", help = "Your display name")]
    pub name: String,

    #[arg(
        long,
        default_value_t = local_url::DEFAULT_API_PORT,
        help = "MCP server port"
    )]
    pub mcp_port: u16,

    #[arg(long, help = "Data base directory (defaults to ~/.bpmux/data)")]
    pub data_base: Option<String>,

    #[command(subcommand)]
    pub command: Option<crate::Command>,
}

pub fn run_recall(cli: &Cli, role_arg: &Option<String>) -> anyhow::Result<()> {
    let role = role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .unwrap_or_default();
    if role.is_empty() {
        eprintln!("bpmux recall: no role specified (pass --role or set BPMUX_ROLE)");
        return Ok(());
    }
    let base = resolve_data_base(&cli.data_base)?;
    let data_dir = match location::durable_data_dir(&base) {
        Ok(dir) => dir,
        Err(_) => return Ok(()),
    };
    let lessons = memory::recall::surface_for_role(&data_dir, &role);
    print!("{}", memory::recall::format_for_surface(&role, &lessons));
    crate::emit_recall_activity(&role, lessons.len());
    Ok(())
}
