mod identity;
mod messaging;
mod planning;
mod query;
mod read;
mod story;
mod workflow;

use clap::Subcommand;

use self::workflow::{WorkflowCommands, dispatch_workflow};
use super::actions::RoomAction;
use super::messaging::run_emit_activity;
use identity::{IdentityCommands, dispatch_identity};
use messaging::{MessagingCommands, dispatch_messaging};
use planning::{PlanningCommands, dispatch_planning};
use query::{QueryCommands, dispatch_query};
use read::{ReadCommands, dispatch_read};
use story::{StoryCommands, dispatch_story};

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Run as a channel bridge for Claude Code integration")]
    Bridge {
        #[arg(
            long,
            default_value_t = local_url::DEFAULT_API_PORT,
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
        about = "Emit an activity-class broadcast message attributed to an arbitrary sender. The generic substrate affordance scripts/hooks use to post automation signals into the bpmux stream (e.g. the git post-commit hook emits pre-commit-gates-passed). The substrate stays content-agnostic: the sender label and wording are the caller's concern."
    )]
    EmitActivity {
        #[arg(
            long,
            help = "Sender label to attribute the activity to (e.g. \"dev:pre-commit-hook\")."
        )]
        from: String,
        #[arg(long, help = "The activity message text.")]
        text: String,
    },
    Room {
        #[command(subcommand)]
        action: RoomAction,
    },
    #[command(
        about = "Migrate the existing jsonl message log into the SQLite store before cutover (#1799). Idempotent; drops the ephemeral board_snapshot deltas (#1642); leaves the jsonl as source-of-truth until you verify and flip BPMUX_STORAGE_BACKEND=sqlite."
    )]
    MigrateStorage,
    #[command(flatten)]
    Messaging(MessagingCommands),
    #[command(flatten)]
    Read(ReadCommands),
    #[command(flatten)]
    Workflow(WorkflowCommands),
    #[command(flatten)]
    Story(StoryCommands),
    #[command(flatten)]
    Planning(PlanningCommands),
    #[command(flatten)]
    Identity(IdentityCommands),
    #[command(flatten)]
    Query(QueryCommands),
}

pub fn dispatch(command: &Command) -> Option<anyhow::Result<()>> {
    match command {
        Command::EmitActivity { from, text } => Some(run_emit_activity(from, text)),
        Command::MigrateStorage => Some(super::migrate::run_migrate_storage()),
        Command::Messaging(c) => Some(dispatch_messaging(c)),
        Command::Read(c) => Some(dispatch_read(c)),
        Command::Workflow(c) => Some(dispatch_workflow(c)),
        Command::Story(c) => Some(dispatch_story(c)),
        Command::Planning(c) => Some(dispatch_planning(c)),
        Command::Identity(c) => Some(dispatch_identity(c)),
        Command::Query(c) => Some(dispatch_query(c)),
        Command::Bridge { .. }
        | Command::Recall { .. }
        | Command::Room { .. }
        | Command::Server => None,
    }
}
