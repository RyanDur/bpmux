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
    #[serde(default)]
    pub conflicts_pending: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroppedEvent {
    pub story_number: u64,
    pub error: String,
    pub ts: DateTime<Utc>,
}

pub struct SyncTransition {
    pub status: &'static str,
    pub why: String,
    pub pending_events: usize,
    pub conflicts_pending: usize,
}

pub fn sync_transition(snapshot: &SyncStatus) -> Option<SyncTransition> {
    let status = if snapshot.conflicts_pending > 0 {
        "conflict"
    } else if snapshot.stale {
        "stale"
    } else if snapshot.pending_events > 0 {
        "drain"
    } else {
        return None;
    };
    let why = match status {
        "conflict" => format!(
            "{} unresolved label conflict(s) pending operator review",
            snapshot.conflicts_pending
        ),
        "stale" => snapshot.last_error.clone().unwrap_or_else(|| {
            format!(
                "no successful ingest within the {}s staleness threshold",
                staleness_threshold_secs()
            )
        }),
        _ => format!(
            "{} board event(s) queued for drain",
            snapshot.pending_events
        ),
    };
    Some(SyncTransition {
        status,
        why,
        pending_events: snapshot.pending_events,
        conflicts_pending: snapshot.conflicts_pending,
    })
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
