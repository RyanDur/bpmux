use clap::Subcommand;

use crate::cli::write_story::{run_close, run_comment, run_create, run_edit, run_label, run_move};

#[derive(Subcommand)]
pub(crate) enum StoryCommands {
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
        #[arg(
            long,
            help = "Acceptor override: team | owner | dev (overrides the use-axis derivation when it can't classify the axis or gets it wrong)"
        )]
        acceptor: Option<String>,
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
}

pub(crate) fn dispatch_story(command: &StoryCommands) -> anyhow::Result<()> {
    match command {
        StoryCommands::Move {
            number,
            target_state,
            rationale,
            position,
        } => run_move(*number, target_state, rationale, *position),
        StoryCommands::Label {
            number,
            label,
            operation,
        } => run_label(*number, label, operation),
        StoryCommands::Create {
            title,
            user_job,
            observation_falsifier,
            body,
            parent_epic,
            acceptance_test_path,
            acceptor,
        } => run_create(
            title,
            user_job,
            observation_falsifier,
            body,
            *parent_epic,
            acceptance_test_path,
            acceptor,
        ),
        StoryCommands::Edit {
            number,
            title,
            body,
        } => run_edit(*number, title, body),
        StoryCommands::Comment { number, body } => run_comment(*number, body),
        StoryCommands::Close {
            number,
            reason,
            disposition,
        } => run_close(*number, reason, disposition),
    }
}
