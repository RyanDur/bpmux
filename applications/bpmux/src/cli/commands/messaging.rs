use clap::Subcommand;

use crate::cli::help::run_help;
use crate::cli::messaging::{run_read, run_register, run_send, run_wait_for_event};

#[derive(Subcommand)]
pub(crate) enum MessagingCommands {
    #[command(
        about = "Block until this agent's next team-event arrives, draining bpmux's own store off Claude's Channels (Stop-hook receive-loop). Prints the event JSON or a timed-out result to stdout; identity is ambient via BPMUX_ROLE."
    )]
    WaitForEvent {
        #[arg(
            long,
            help = "Only return events newer than this cursor (from a prior call's cursor field); advances so the same event is not re-delivered"
        )]
        after: Option<String>,
        #[arg(
            long,
            default_value_t = crate::transport::rest::DEFAULT_WAIT_TIMEOUT_MS,
            help = "Max milliseconds to block before returning a timed-out result"
        )]
        timeout_ms: u64,
    },
    #[command(
        about = "Send a team message to the bpmux channel as this agent (ambient role via BPMUX_ROLE), off Claude's MCP. Renders the command + a one-line confirm in the terminal stream."
    )]
    Send {
        #[arg(help = "The message text to send")]
        text: String,
        #[arg(
            long,
            value_delimiter = ',',
            help = "Comma-separated recipient roles (privacy filter); omit for broadcast"
        )]
        to: Vec<String>,
        #[arg(long, help = "Role this message is directed at (attention signal)")]
        addressee: Option<String>,
    },
    #[command(
        about = "Register this agent's routing identity at startup (channels-independence #1712). Derives a stable session-id from the current directory (reclaimed across restart-in-the-same-dir, else minted) and registers it with the server keyed to this agent's display-name (ambient role via BPMUX_ROLE). Agnostic: a direct server call, no Claude-specific glue."
    )]
    Register,
    #[command(
        about = "Read recent team messages as a compact index (sender + one-line), off Claude's MCP. Use --full for the raw payload."
    )]
    Read {
        #[arg(long, default_value_t = 20, help = "Max messages to show")]
        limit: usize,
        #[arg(
            long,
            help = "Emit the raw message payload instead of the compact index"
        )]
        full: bool,
    },
    #[command(
        about = "List the bpmux verbs for discovery (MCP advertises tools for free; the CLI re-creates discovery here). --json emits a machine-readable verb-index an agent can parse."
    )]
    Help {
        #[arg(long, help = "Emit the verb-index as JSON for agent parsing")]
        json: bool,
    },
}

pub(crate) fn dispatch_messaging(command: &MessagingCommands) -> anyhow::Result<()> {
    match command {
        MessagingCommands::WaitForEvent { after, timeout_ms } => {
            run_wait_for_event(after, *timeout_ms)
        }
        MessagingCommands::Send {
            text,
            to,
            addressee,
        } => run_send(text, to, addressee),
        MessagingCommands::Register => run_register(),
        MessagingCommands::Read { limit, full } => run_read(*limit, *full),
        MessagingCommands::Help { json } => run_help(*json),
    }
}
