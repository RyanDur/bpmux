use super::CiSnapshot;
use crate::orchestrator::stores::{ReactionsAction, RosterAction};
use crate::product_management::board::MutationId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PollKind {
    Ci,
    Ingest,
    Hydration,
}

#[derive(Clone, Debug)]
pub enum OrchestratorAction {
    StartCiPolling,
    StopCiPolling,
    CiUpdated(CiSnapshot),
    StartIngestPolling,
    StopIngestPolling,
    StartHydrationPolling,
    StopHydrationPolling,
    Reactions(ReactionsAction),
    Roster(RosterAction),
    StartUpdateGithub {
        mutation_id: MutationId,
    },
    StopUpdateGithub {
        mutation_id: MutationId,
    },
    MutationConfirmed {
        mutation_id: MutationId,
    },
    MutationRolledBack {
        mutation_id: MutationId,
        server_story: Option<Box<crate::product_management::story::Story>>,
    },
}

pub type CiAction = OrchestratorAction;

pub(super) fn validate_and_log(action: &OrchestratorAction) {
    let label = match action {
        OrchestratorAction::StartCiPolling => "StartCiPolling",
        OrchestratorAction::StopCiPolling => "StopCiPolling",
        OrchestratorAction::CiUpdated(_) => "CiUpdated",
        OrchestratorAction::StartIngestPolling => "StartIngestPolling",
        OrchestratorAction::StopIngestPolling => "StopIngestPolling",
        OrchestratorAction::StartHydrationPolling => "StartHydrationPolling",
        OrchestratorAction::StopHydrationPolling => "StopHydrationPolling",
        OrchestratorAction::Reactions(_) => "Reactions",
        OrchestratorAction::Roster(_) => "Roster",
        OrchestratorAction::StartUpdateGithub { .. } => "StartUpdateGithub",
        OrchestratorAction::StopUpdateGithub { .. } => "StopUpdateGithub",
        OrchestratorAction::MutationConfirmed { .. } => "MutationConfirmed",
        OrchestratorAction::MutationRolledBack { .. } => "MutationRolledBack",
    };
    crate::diag::log_diag(&format!("orchestrator: dispatch {label}"));
}
