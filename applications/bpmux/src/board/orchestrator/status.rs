//! Sync status: the observable health surface of `SyncOrchestrator`
//! (last ingest/push timestamps, pending-event count, errors, drops,
//! conflicts). Plus the transactional-closure record-* helpers that
//! all status mutations route through (#1360 — single canonical
//! lock-acquire site).

use std::sync::Arc;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncStatus {
    pub last_ingest_at: Option<DateTime<Utc>>,
    pub last_push_at: Option<DateTime<Utc>>,
    pub pending_events: usize,
    pub last_error: Option<String>,
    pub dropped_events: Vec<DroppedEvent>,
    pub recovering_from_corruption: bool,
    #[serde(default)]
    pub stale: bool,
    #[serde(default)]
    pub startup_fail: bool,
    /// #1195: per-op count of currently-conflicted outbox entries.
    /// Each drain tick re-classifies; a conflicted entry (server
    /// state diverged externally between mutate-time and drain-time)
    /// stays in the outbox + bumps this count for SyncStatusGlyph
    /// state #5 (slice B) to render. Idempotent skips do NOT count
    /// (they get dropped + diag-logged). Auto-resolved on next
    /// drain tick when the divergent server state catches up.
    #[serde(default)]
    pub conflicts_pending: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroppedEvent {
    pub story_number: u64,
    pub error: String,
    pub ts: DateTime<Utc>,
}

pub fn staleness_threshold_secs() -> u64 {
    std::env::var("BPMUX_STALENESS_THRESHOLD_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(180)
}

pub fn is_stale(
    last_ingest_at: Option<DateTime<Utc>>,
    threshold_secs: u64,
    now: DateTime<Utc>,
) -> bool {
    let Some(last) = last_ingest_at else {
        return false;
    };
    let elapsed = now.signed_duration_since(last);
    elapsed.num_seconds() >= 0 && (elapsed.num_seconds() as u64) > threshold_secs
}

pub fn is_startup_fail(last_ingest_at: Option<DateTime<Utc>>, last_error: Option<&str>) -> bool {
    last_ingest_at.is_none() && last_error.is_some()
}

/// Transactional-closure mutation over a `SyncStatus` behind an
/// `Arc<Mutex<_>>`. Free-fn form so the standalone `record_*` /
/// `mark_*` helpers below can route their lock-acquire through one
/// canonical site instead of each holding its own. Per #1360: lock-
/// scope bounded + visible at the call-site, not scattered.
pub(super) fn with_status_mut<R>(
    status: &Arc<Mutex<SyncStatus>>,
    f: impl FnOnce(&mut SyncStatus) -> R,
) -> R {
    let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut s)
}

pub(super) fn record_ingest_success(status: &Arc<Mutex<SyncStatus>>) {
    with_status_mut(status, |s| {
        s.last_ingest_at = Some(Utc::now());
        s.last_error = None;
        s.recovering_from_corruption = false;
    });
}

pub fn mark_recovering_from_corruption(status: &Arc<Mutex<SyncStatus>>) {
    with_status_mut(status, |s| {
        s.recovering_from_corruption = true;
    });
}

pub(super) fn record_push_success(status: &Arc<Mutex<SyncStatus>>) {
    with_status_mut(status, |s| {
        s.last_push_at = Some(Utc::now());
        if s.pending_events > 0 {
            s.pending_events -= 1;
        }
    });
}

pub(super) fn record_error(status: &Arc<Mutex<SyncStatus>>, msg: &str) {
    with_status_mut(status, |s| {
        s.last_error = Some(msg.to_string());
    });
}

pub(super) fn set_pending_events(status: &Arc<Mutex<SyncStatus>>, value: usize) {
    with_status_mut(status, |s| {
        s.pending_events = value;
    });
}

pub(super) fn decrement_pending(status: &Arc<Mutex<SyncStatus>>) {
    with_status_mut(status, |s| {
        if s.pending_events > 0 {
            s.pending_events -= 1;
        }
    });
}

pub(super) fn record_dropped(status: &Arc<Mutex<SyncStatus>>, story_number: u64, error: String) {
    with_status_mut(status, |s| {
        s.dropped_events.push(DroppedEvent {
            story_number,
            error,
            ts: Utc::now(),
        });
    });
}

/// #1195: write the count of currently-conflicted outbox entries
/// into SyncStatus. Each drain tick re-runs the classifier on
/// every pending entry, so this fully reflects the current outbox
/// state — no incremental accumulation across ticks.
pub(super) fn record_conflicts_pending(status: &Arc<Mutex<SyncStatus>>, count: usize) {
    with_status_mut(status, |s| {
        s.conflicts_pending = count;
    });
}

pub fn acknowledge_dropped(status: &Arc<Mutex<SyncStatus>>) -> usize {
    with_status_mut(status, |s| {
        let cleared = s.dropped_events.len();
        s.dropped_events.clear();
        cleared
    })
}
