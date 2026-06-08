use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Command {
    #[command(about = "Run as a channel bridge for Claude Code integration")]
    Bridge {
        #[arg(
            long,
            default_value_t = crate::transport::local_url::DEFAULT_API_PORT,
            help = "bpmux server port to connect to"
        )]
        port: u16,

        #[arg(
            long,
            default_value = "claude-code",
            help = "Base identity name for this bridge (a unique suffix is appended)"
        )]
        name: String,

        #[arg(
            long,
            help = "Use fake story tracker (no gh calls). Test infrastructure only."
        )]
        fake_tracker: bool,

        #[arg(
            long,
            help = "Path to a JSON fixture file for list_stories. Test infrastructure only."
        )]
        board_fixture: Option<String>,
    },
    Server,
    #[command(
        about = "Print an agent's recent lessons to stdout for session-start auto-surface (#157)"
    )]
    Recall {
        #[arg(
            long,
            help = "Role to surface lessons for (e.g. dev, pm, designer). Defaults to BPMUX_ROLE env var if unset."
        )]
        role: Option<String>,
    },
    #[command(
        about = "Emit a pre-commit-gates-passed activity broadcast (#424). Invoked by the git post-commit hook with the SHA of the just-created commit. Fires only on gate-passed state because the commit wouldn't exist if pre-commit had failed."
    )]
    EmitPrecommitPassed {
        #[arg(long, help = "SHA of the commit whose pre-commit gates just passed.")]
        sha: String,
        #[arg(
            long,
            help = "Role to attribute the broadcast to. Defaults to BPMUX_ROLE env var; falls back to \"dev\"."
        )]
        role: Option<String>,
    },
    Room {
        #[command(subcommand)]
        action: RoomAction,
    },
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
        about = "Pass a story's desk-check as this agent (ambient role via BPMUX_ROLE), off Claude's MCP — fires the same gated /api/rpc/pass_desk_check path the bridge uses, so the role-guard, verdict-evidence, and CI-green gates apply identically. --acceptor voices the acceptor-class at the gate."
    )]
    Pass {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Acceptor class voiced at the gate: team | owner | dev")]
        acceptor: String,
    },
}

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
