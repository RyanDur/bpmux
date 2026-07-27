use crate::ci::CiAction;
use crate::commitments::CommitmentsAction;
use crate::routing::RoutingAction;
use crate::sync::SyncAction;
use crate::sync_health::SyncHealthAction;

use super::AppAction;

pub fn validate_and_log(action: &AppAction) {
    let label = match action {
        AppAction::Ci(CiAction::StartPolling) => "Ci.StartPolling",
        AppAction::Ci(CiAction::PollingStarted) => "Ci.PollingStarted",
        AppAction::Ci(CiAction::StopPolling) => "Ci.StopPolling",
        AppAction::Ci(CiAction::PollingStopped) => "Ci.PollingStopped",
        AppAction::Ci(CiAction::Updated(_)) => "Ci.Updated",
        AppAction::Ci(CiAction::PollSucceeded { .. }) => "Ci.PollSucceeded",
        AppAction::Sync(SyncAction::StartIngestPolling) => "Sync.StartIngestPolling",
        AppAction::Sync(SyncAction::StopIngestPolling) => "Sync.StopIngestPolling",
        AppAction::Sync(SyncAction::StartHydrationPolling) => "Sync.StartHydrationPolling",
        AppAction::Sync(SyncAction::StopHydrationPolling) => "Sync.StopHydrationPolling",
        AppAction::Sync(SyncAction::WriteQueued { .. }) => "Sync.WriteQueued",
        AppAction::Sync(SyncAction::WritesQueued(_)) => "Sync.WritesQueued",
        AppAction::Sync(SyncAction::WritePushed { .. }) => "Sync.WritePushed",
        AppAction::Sync(SyncAction::WriteDropped { .. }) => "Sync.WriteDropped",
        AppAction::SyncHealth(SyncHealthAction::IngestPollingStarted) => {
            "SyncHealth.IngestPollingStarted"
        }
        AppAction::SyncHealth(SyncHealthAction::IngestPollingStopped) => {
            "SyncHealth.IngestPollingStopped"
        }
        AppAction::SyncHealth(SyncHealthAction::HydrationPollingStarted) => {
            "SyncHealth.HydrationPollingStarted"
        }
        AppAction::SyncHealth(SyncHealthAction::HydrationPollingStopped) => {
            "SyncHealth.HydrationPollingStopped"
        }
        AppAction::SyncHealth(SyncHealthAction::IngestSucceeded { .. }) => {
            "SyncHealth.IngestSucceeded"
        }
        AppAction::SyncHealth(SyncHealthAction::PushSucceeded { .. }) => "SyncHealth.PushSucceeded",
        AppAction::SyncHealth(SyncHealthAction::ErrorRecorded { .. }) => "SyncHealth.ErrorRecorded",
        AppAction::SyncHealth(SyncHealthAction::EventDropped { .. }) => "SyncHealth.EventDropped",
        AppAction::SyncHealth(SyncHealthAction::DroppedAcknowledged) => {
            "SyncHealth.DroppedAcknowledged"
        }
        AppAction::Commitments(CommitmentsAction::Committed { .. }) => "Commitments.Committed",
        AppAction::Commitments(CommitmentsAction::CommitmentsMatched { .. }) => {
            "Commitments.CommitmentsMatched"
        }
        AppAction::Routing(RoutingAction::Registered { .. }) => "Routing.Registered",
        AppAction::Routing(RoutingAction::TerminalLinked { .. }) => "Routing.TerminalLinked",
        AppAction::Routing(RoutingAction::Renamed { .. }) => "Routing.Renamed",
        AppAction::Routing(RoutingAction::CursorAdvanced { .. }) => "Routing.CursorAdvanced",
        AppAction::Routing(RoutingAction::CursorAnchored { .. }) => "Routing.CursorAnchored",
        AppAction::Routing(RoutingAction::Reaped { .. }) => "Routing.Reaped",
        AppAction::Reactions(_) => "Reactions",
        AppAction::Evidence(_) => "Evidence",
        AppAction::Roster(_) => "Roster",
        AppAction::Outcome(_) => "Outcome",
        AppAction::Iteration(_) => "Iteration",
        AppAction::Epic(_) => "Epic",
        AppAction::Story(_) => "Story",
        AppAction::Board(_) => "Board",
        AppAction::SharedHistory(_) => "SharedHistory",
        AppAction::OwnerSession(_) => "OwnerSession",
        AppAction::ClientConnected(_) => "ClientConnected",
    };
    match action {
        AppAction::Routing(RoutingAction::Registered {
            user_id, display, ..
        }) => diag::log_diag(&format!(
            "store: dispatch {label} id={user_id} display={display}"
        )),
        AppAction::Routing(RoutingAction::TerminalLinked {
            user_id,
            terminal_id,
        }) => diag::log_diag(&format!(
            "store: dispatch {label} id={user_id} terminal={terminal_id}"
        )),
        AppAction::Routing(RoutingAction::Reaped { display }) => {
            diag::log_diag(&format!("store: dispatch {label} display={display}"))
        }
        _ => diag::log_diag(&format!("store: dispatch {label}")),
    }
}
