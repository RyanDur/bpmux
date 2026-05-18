//! Conflict classification for outbox-drain: the 5 ConflictKind
//! outcomes the classifier compares (pre_state, event, current)
//! against, plus the ConflictResolveOrigin tag that records HOW a
//! conflicted entry got auto-resolved.
//!
//! Slice A ships the substrate (types + the always-None classifier);
//! slice B+ plugs in the real logic that compares intended-vs-server
//! truth without re-fetching from GH per #1194's no-per-op-GH-GET.

use serde::{Deserialize, Serialize};

use crate::board::domain::Story;
use crate::board::events::BoardEvent;

/// #1195 conflict classification result. The classifier compares
/// `pre_state` (snapshot captured at mutate-time) → `event` (the
/// intended mutation) → `current` (canonical-store-state at
/// drain-time, NOT GH state per #1194's no-per-op-GH-GET
/// discipline). Five outcomes:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictKind {
    /// The mutation is already applied at the canonical store and
    /// would be a no-op (label-already-present, already-closed,
    /// parent-epic-already-pointing-here). Drop without firing GH.
    IdempotentSkip,
    /// Server state diverged from pre_state in a way that conflicts
    /// with the event's intended state-transition (e.g., offline
    /// `StateTransitioned → Ready` was queued when server was
    /// `Delivered`, but server is now at `Accepted`). Operator
    /// click-to-resolve via slice B.
    DivergentState,
    /// Server label set diverged from pre_state in a way that
    /// conflicts with the event's intended label change. Operator
    /// click-to-resolve via slice B.
    DivergentLabel,
    /// External mutation arrived between queue-time and drain-time
    /// and the server-shifted state is now incompatible with the
    /// queued event's basic preconditions (e.g., we queued
    /// `TagAdded` against an open issue but server has since closed
    /// it). Sister to Divergent* but generalized.
    External,
}

/// #1195: 3-arg conflict classifier. Compares pre-mutation
/// snapshot (captured at queue-time) against the event's intended
/// mutation against the current canonical-store state. Pure
/// function — no I/O. Returns `None` when the event is fine to
/// push as-is.
///
/// Per #1194's no-per-op-GH-GET shipped substrate, `current` is
/// the canonical-store-state, NOT a fresh GH fetch. The ingest
/// poller (60s default) is what brings server-truth into the
/// canonical store; this function trusts that view.
///
/// `pre_state: None` means: legacy outbox entry from before #1195
/// OR the story didn't exist in the store at queue-time. The
/// classifier degrades to no-conflict-detection (returns None) in
/// that case — we can't tell if the world changed without the
/// snapshot, so we err on "push and let the adapter surface any
/// error".
///
/// Current slice ships SUBSTRATE only — types + queue-time pre_state
/// capture + ConflictDetected/Resolved events + the SyncStatus
/// .conflicts_pending field + outbox schema migration. The classifier
/// returns None for now because correct semantics require tracking
/// cumulative-local-mutation-effects separately from GH-fetched state
/// (raw_labels reflects fetch-state, not local-applied state until
/// LabelsRefreshed roundtrips). That scope-narrows the comparison to
/// "intended-vs-server-truth" which is slice B+ work. The substrate
/// IS in place so slice B plugs in the real classifier without further
/// migrations. Annotation/observation events (StoryCreated /
/// GateCommentAppended / etc.) are additive and not conflict-
/// classifiable in any slice.
pub fn classify_conflict(
    _pre_state: Option<&Story>,
    event: &BoardEvent,
    _current: Option<&Story>,
) -> Option<ConflictKind> {
    match event {
        BoardEvent::TagAdded { .. }
        | BoardEvent::TagRemoved { .. }
        | BoardEvent::StateTransitioned { .. }
        | BoardEvent::StoryClosed { .. }
        | BoardEvent::StoryReopened { .. }
        | BoardEvent::ParentEpicSet { .. }
        | BoardEvent::ParentEpicUnset { .. } => None,
        BoardEvent::StoryCreated { .. }
        | BoardEvent::GateCommentAppended { .. }
        | BoardEvent::FreeCommentAppended { .. }
        | BoardEvent::BodyFetched { .. }
        | BoardEvent::FreeCommentsFetched { .. }
        | BoardEvent::GateCommentsFetched { .. }
        | BoardEvent::StoryRefreshed { .. }
        | BoardEvent::LabelsRefreshed { .. }
        | BoardEvent::ConflictDetected { .. }
        | BoardEvent::ConflictResolved { .. } => None,
    }
}

/// #1195: origin tag for ConflictResolved events. Slice A only
/// fires the auto-reclassify origin (drain detects compatibility);
/// slice B adds OperatorAction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolveOrigin {
    DrainAutoReclassify,
    OperatorAction,
}
