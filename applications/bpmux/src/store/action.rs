use super::ci::CiSnapshot;
use crate::product_management::board::MutationId;
use crate::store::stores::{ReactionsAction, RosterAction};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PollKind {
    Ci,
    Ingest,
    Hydration,
}

#[derive(Clone, Debug)]
pub enum StoreAction {
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

pub type CiAction = StoreAction;

pub(super) fn validate_and_log(action: &StoreAction) {
    let label = match action {
        StoreAction::StartCiPolling => "StartCiPolling",
        StoreAction::StopCiPolling => "StopCiPolling",
        StoreAction::CiUpdated(_) => "CiUpdated",
        StoreAction::StartIngestPolling => "StartIngestPolling",
        StoreAction::StopIngestPolling => "StopIngestPolling",
        StoreAction::StartHydrationPolling => "StartHydrationPolling",
        StoreAction::StopHydrationPolling => "StopHydrationPolling",
        StoreAction::Reactions(_) => "Reactions",
        StoreAction::Roster(_) => "Roster",
        StoreAction::StartUpdateGithub { .. } => "StartUpdateGithub",
        StoreAction::StopUpdateGithub { .. } => "StopUpdateGithub",
        StoreAction::MutationConfirmed { .. } => "MutationConfirmed",
        StoreAction::MutationRolledBack { .. } => "MutationRolledBack",
    };
    crate::diag::log_diag(&format!("store: dispatch {label}"));
}
