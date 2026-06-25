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
    Ci(CiAction),
    Sync(SyncAction),
    Reactions(ReactionsAction),
    Roster(RosterAction),
}

#[derive(Clone, Debug)]
pub enum CiAction {
    StartPolling,
    StopPolling,
    Updated(CiSnapshot),
}

#[derive(Clone, Debug)]
pub enum SyncAction {
    StartIngestPolling,
    StopIngestPolling,
    StartHydrationPolling,
    StopHydrationPolling,
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

pub(super) fn validate_and_log(action: &StoreAction) {
    let label = match action {
        StoreAction::Ci(CiAction::StartPolling) => "Ci.StartPolling",
        StoreAction::Ci(CiAction::StopPolling) => "Ci.StopPolling",
        StoreAction::Ci(CiAction::Updated(_)) => "Ci.Updated",
        StoreAction::Sync(SyncAction::StartIngestPolling) => "Sync.StartIngestPolling",
        StoreAction::Sync(SyncAction::StopIngestPolling) => "Sync.StopIngestPolling",
        StoreAction::Sync(SyncAction::StartHydrationPolling) => "Sync.StartHydrationPolling",
        StoreAction::Sync(SyncAction::StopHydrationPolling) => "Sync.StopHydrationPolling",
        StoreAction::Sync(SyncAction::StartUpdateGithub { .. }) => "Sync.StartUpdateGithub",
        StoreAction::Sync(SyncAction::StopUpdateGithub { .. }) => "Sync.StopUpdateGithub",
        StoreAction::Sync(SyncAction::MutationConfirmed { .. }) => "Sync.MutationConfirmed",
        StoreAction::Sync(SyncAction::MutationRolledBack { .. }) => "Sync.MutationRolledBack",
        StoreAction::Reactions(_) => "Reactions",
        StoreAction::Roster(_) => "Roster",
    };
    crate::diag::log_diag(&format!("store: dispatch {label}"));
}
