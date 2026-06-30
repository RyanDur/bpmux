use crate::product_management::outcome::OutcomeAction;
use crate::store::{CiAction, ReactionsAction, RosterAction, SyncAction};

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
    Outcome(OutcomeAction),
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
        StoreAction::Outcome(_) => "Outcome",
    };
    crate::diag::log_diag(&format!("store: dispatch {label}"));
}
