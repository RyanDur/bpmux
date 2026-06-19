mod identity;
mod messaging;
mod planning;
mod query;
mod read;
mod story;
mod workflow;

use clap::Subcommand;

use super::actions::RoomAction;
use super::messaging::run_emit_precommit_passed;
use identity::{IdentityCommands, dispatch_identity};
use messaging::{MessagingCommands, dispatch_messaging};
use planning::{PlanningCommands, dispatch_planning};
use query::{QueryCommands, dispatch_query};
use read::{ReadCommands, dispatch_read};
use story::{StoryCommands, dispatch_story};
use workflow::{WorkflowCommands, dispatch_workflow};

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

pub(crate) fn dispatch(command: &Command) -> Option<anyhow::Result<()>> {
    match command {
        Command::EmitPrecommitPassed { sha, role } => Some(run_emit_precommit_passed(sha, role)),
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
