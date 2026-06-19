use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum RoomAction {
    Spawn {
        role: String,
        #[arg(long, default_value = "claude-code")]
        agent: String,
        #[arg(long)]
        headless: bool,
    },
    List {},
    Read {
        role: String,
        #[arg(long)]
        lines: Option<usize>,
    },
    Kill {
        role: String,
    },
}
