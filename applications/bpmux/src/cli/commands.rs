use clap::Subcommand;

use super::actions::{EpicAction, IterationAction, LessonAction, RoomAction};

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
        about = "Capture the bpmux Electron renderer window as a PNG off Claude's MCP — fires the same gated /api/electron-capture path the MCP capture_electron_window tool uses, returning the byte-same artifact (JSON `{path, window_id, captured_at, width, height}` under ~/.bpmux/captures). The Electron window is the only live capture surface; the TUI screen-capture was retired in #1166."
    )]
    Capture {
        #[arg(
            long,
            help = "Capture the Electron renderer window (required — the live walk surface; the TUI screen-capture was retired in #1166)"
        )]
        window: bool,
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
    #[command(
        about = "Start (pick up) a ready story as this agent off Claude's MCP — fires the same gated /api/rpc/start_story path the bridge uses (dev-only pickup role-guard; owner bypasses via the #1548 superset)."
    )]
    Start {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Your restatement of the user-job at pickup")]
        paraphrase: String,
        #[arg(long, help = "Your reframing of the observation-falsifier")]
        falsifier: String,
    },
    #[command(
        about = "Mark a story delivered as this agent off Claude's MCP — fires the same gated /api/rpc/deliver_story path the bridge uses (dev-only; owner bypasses)."
    )]
    Deliver {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "What you drove end-to-end (the desk-check diff-point)")]
        observation: String,
    },
    #[command(
        about = "Fail a story's desk-check as this agent off Claude's MCP — fires the same gated /api/rpc/fail_desk_check path the bridge uses (pm/designer role-guard; owner bypasses)."
    )]
    Fail {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "What failed — surfaced to dev as the workflow signal")]
        reason: String,
    },
    #[command(
        about = "Reject a delivered story as this agent off Claude's MCP — fires the same gated /api/rpc/reject_story path the bridge uses (pm role-guard; owner bypasses)."
    )]
    Reject {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Rejection reason — recorded as a comment on the story")]
        reason: Option<String>,
    },
    #[command(
        about = "Cast a substrate-grounded verdict on a story off Claude's MCP — fires the same gated /api/rpc/submit_verdict handler the bridge uses (dev/designer-only guard; the owner is not an evidence role and is refused, identical to MCP)."
    )]
    Verdict {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Verdict outcome, e.g. verified | rejected")]
        verdict: String,
        #[arg(
            long,
            help = "Required source-grounded cite for the verdict (non-empty)"
        )]
        grounding: String,
    },
    #[command(
        about = "Accept a delivered story off Claude's MCP — fires the same gated /api/rpc/accept_story handler the bridge uses (role-guard by gate-class + evidence + CI + bug-question), NOT the gate-skipping Electron-accept REST. The owner bypasses the role-guard via the #1548 superset."
    )]
    Accept {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(
            long,
            help = "Bug-question: would you file a bug if you accepted this as-is? (blocks unless --force + --bypass-reason)"
        )]
        would_file_bug: bool,
        #[arg(
            long,
            help = "Override the bug-question block (requires --bypass-reason)"
        )]
        force: bool,
        #[arg(
            long,
            help = "Justification recorded in the gate-comment when --force overrides the bug-question"
        )]
        bypass_reason: Option<String>,
    },
    #[command(
        about = "Move a story to a column (and optionally reorder) off Claude's MCP — fires the same gated /api/rpc/move_story path the bridge uses (PM-only role-guard for all moves; owner bypasses via the #1548 superset)."
    )]
    Move {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(
            long,
            help = "Target column: icebox | ready | in-progress | desk-check | delivered | accepted | retired"
        )]
        target_state: String,
        #[arg(long, help = "Why the move — the required audit-trail reason")]
        rationale: String,
        #[arg(long, help = "1-based position for ready/icebox ordering (optional)")]
        position: Option<usize>,
    },
    #[command(
        about = "Add or remove a label on a story off Claude's MCP — fires the same gated /api/rpc/label_story path the bridge uses (PM-only role-guard; owner bypasses via the #1548 superset)."
    )]
    Label {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "The label to add or remove")]
        label: String,
        #[arg(long, help = "add (default) | remove")]
        operation: Option<String>,
    },
    #[command(
        about = "Create a story off Claude's MCP — fires the same gated /api/rpc/create_story handler the bridge uses (pm-lane: pm any type, dev [Chore]-class only; owner bypasses via the #1548 superset)."
    )]
    Create {
        #[arg(long, help = "Story title")]
        title: String,
        #[arg(
            long,
            help = "The user-job (Mike Cohn): as <role>, I want <goal> so that <benefit>"
        )]
        user_job: String,
        #[arg(
            long,
            help = "The observation-falsifier — what would make you reject this as built"
        )]
        observation_falsifier: String,
        #[arg(long, help = "Story body (optional)")]
        body: Option<String>,
        #[arg(long, help = "Parent epic number (optional)")]
        parent_epic: Option<u64>,
        #[arg(long, help = "Acceptance-test path (optional)")]
        acceptance_test_path: Option<String>,
    },
    #[command(
        about = "Edit a story's title/body off Claude's MCP — fires the same gated /api/rpc/edit_story path the bridge uses (PM-only; owner bypasses via the #1709 require_role superset-extension)."
    )]
    Edit {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "New title (optional)")]
        title: Option<String>,
        #[arg(long, help = "New body (optional)")]
        body: Option<String>,
    },
    #[command(
        about = "Comment on a story off Claude's MCP — fires the same gated /api/rpc/comment_on_story path the bridge uses (all roles; no role-restriction)."
    )]
    Comment {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "The comment body")]
        body: String,
    },
    #[command(
        about = "Close a story off Claude's MCP — fires the same gated /api/rpc/close_story path the bridge uses (PM-only via the close transition; owner bypasses via the #1548 superset)."
    )]
    Close {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Why the story is closed")]
        reason: String,
        #[arg(long, help = "retire (default) | wontfix")]
        disposition: Option<String>,
    },
    #[command(
        about = "Iteration operations off Claude's MCP — fire the same gated /api/rpc/{create_iteration,designate_current_iteration} the bridge uses (PM-only via require_role; owner bypasses via the #1709 superset-extension)."
    )]
    Iteration {
        #[command(subcommand)]
        action: IterationAction,
    },
    #[command(
        about = "Epic operations off Claude's MCP — group/ungroup a child story under an Epic via the same gated /api/rpc/{set_parent_epic,unset_parent_epic} the bridge uses (PM-only via require_role; owner bypasses via the #1709 superset)."
    )]
    Epic {
        #[command(subcommand)]
        action: EpicAction,
    },
    #[command(
        about = "Lesson operations off Claude's MCP — save/list lessons via the same /api/lessons + /api/peer-lessons endpoints the bridge uses (ungated; author_role from BPMUX_ROLE)."
    )]
    Lesson {
        #[command(subcommand)]
        action: LessonAction,
    },
}
