use clap::Subcommand;

use crate::cli::actions::{EpicAction, IterationAction, OutcomeAction};
use crate::cli::write_iteration::{
    run_epic_children, run_epic_set_parent, run_epic_unset_parent, run_iteration_add,
    run_iteration_close, run_iteration_close_review, run_iteration_create, run_iteration_designate,
    run_iteration_dev_ok, run_outcome_add, run_outcome_create, run_outcome_reprioritize,
};

#[derive(Subcommand)]
pub(crate) enum PlanningCommands {
    #[command(
        about = "Iteration operations off Claude's MCP — fire the same gated /api/rpc/{create_iteration,designate_current_iteration} the bridge uses (PM-only via require_role; owner bypasses via the #1709 superset-extension)."
    )]
    Iteration {
        #[command(subcommand)]
        action: IterationAction,
    },
    #[command(
        about = "Outcome operations off Claude's MCP — create first-class roadmap outcomes (the why, horizon-sequenced) via the gated /api/rpc/create_outcome the bridge uses (PM-only; owner bypasses)."
    )]
    Outcome {
        #[command(subcommand)]
        action: OutcomeAction,
    },
    #[command(
        about = "Epic operations off Claude's MCP — group/ungroup a child story under an Epic via the same gated /api/rpc/{set_parent_epic,unset_parent_epic} the bridge uses (PM-only via require_role; owner bypasses via the #1709 superset)."
    )]
    Epic {
        #[command(subcommand)]
        action: EpicAction,
    },
}

pub(crate) fn dispatch_planning(command: &PlanningCommands) -> anyhow::Result<()> {
    match command {
        PlanningCommands::Iteration { action } => match action {
            IterationAction::Create {
                theme,
                position_after,
            } => run_iteration_create(theme, *position_after),
            IterationAction::Designate { number } => run_iteration_designate(*number),
            IterationAction::Add {
                iteration,
                story,
                position,
            } => run_iteration_add(*iteration, *story, *position),
            IterationAction::Close { iteration } => run_iteration_close(*iteration),
            IterationAction::CloseReview { number } => run_iteration_close_review(*number),
            IterationAction::DevOk { number } => run_iteration_dev_ok(*number),
        },
        PlanningCommands::Outcome { action } => match action {
            OutcomeAction::Create {
                title,
                horizon,
                body,
            } => run_outcome_create(title, horizon, body.as_deref()),
            OutcomeAction::Add {
                outcome,
                iteration,
                position,
            } => run_outcome_add(*outcome, *iteration, *position),
            OutcomeAction::Reprioritize {
                number,
                to,
                position,
            } => run_outcome_reprioritize(*number, to.as_deref(), *position),
        },
        PlanningCommands::Epic { action } => match action {
            EpicAction::SetParent { number, epic } => run_epic_set_parent(*number, *epic),
            EpicAction::UnsetParent { number } => run_epic_unset_parent(*number),
            EpicAction::Children { epic } => run_epic_children(*epic),
        },
    }
}
