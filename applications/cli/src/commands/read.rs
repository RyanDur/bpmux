use clap::Subcommand;

use crate::read::{run_board, run_capture, run_ready, run_story};

#[derive(Subcommand)]
pub enum ReadCommands {
    #[command(
        about = "List the ready queue as a compact scannable index (position, #number, title, acceptor) from bpmux's own store off Claude's MCP — never a stream-flooding dump. --json for machine-parse, --full for the raw board payload."
    )]
    Ready {
        #[arg(long, help = "Emit the ready stories as JSON for agent parsing")]
        json: bool,
        #[arg(long, help = "Emit the raw board payload instead of the compact index")]
        full: bool,
    },
    #[command(
        about = "Show one story as a compact summary (number, title, state, acceptor) from bpmux's own store off Claude's MCP. --json for machine-parse, --full for the body."
    )]
    Story {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Emit the story as JSON for agent parsing")]
        json: bool,
        #[arg(long, help = "Emit the full story body instead of the compact summary")]
        full: bool,
    },
    #[command(
        about = "List the board as a compact scannable index (counts + one-line titles) from bpmux's own store off Claude's MCP — never a stream-flooding dump. --state filters, --limit caps, --json machine-parses, --full emits the raw payload."
    )]
    Board {
        #[arg(
            long,
            help = "Filter to a workflow state (ready, in-progress, desk-check, delivered, acceptance, accepted)"
        )]
        state: Option<String>,
        #[arg(long, help = "Cap the number of stories shown")]
        limit: Option<usize>,
        #[arg(long, help = "Emit the stories as JSON for agent parsing")]
        json: bool,
        #[arg(long, help = "Emit the raw board payload instead of the compact index")]
        full: bool,
    },
    #[command(
        about = "Capture the bpmux Electron renderer window as a PNG off Claude's MCP — fires the same gated /api/electron-capture path the MCP capture_electron_window tool uses, returning the byte-same artifact (JSON `{path, window_id, captured_at, width, height}` under ~/.bpmux/captures). The Electron window is the only live capture surface; the TUI screen-capture was retired in #1166."
    )]
    Capture {
        #[arg(
            long,
            help = "Capture the Electron renderer window (required — the live walk surface; the TUI screen-capture was retired in #1166)"
        )]
        window: bool,
    },
}

pub fn dispatch_read(command: &ReadCommands) -> anyhow::Result<()> {
    match command {
        ReadCommands::Ready { json, full } => run_ready(*json, *full),
        ReadCommands::Story { number, json, full } => run_story(*number, *json, *full),
        ReadCommands::Board {
            state,
            limit,
            json,
            full,
        } => run_board(state, *limit, *json, *full),
        ReadCommands::Capture { window } => run_capture(*window),
    }
}
