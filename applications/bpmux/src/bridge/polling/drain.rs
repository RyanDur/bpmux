use std::sync::Arc;

pub(crate) async fn run_drain_loop(
    refresh: Arc<dyn crate::sync::RefreshDriver>,
    session: Arc<crate::session::Session>,
    interval: std::time::Duration,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let state_changed = match refresh.drain_pending().await {
            Ok(0) => false,
            Ok(n) => {
                crate::diag::log_diag(&format!("bridge: background drain pushed {n} event(s)"));
                true
            }
            Err(e) => {
                crate::diag::log_diag(&format!("bridge: background drain: {e}"));
                true
            }
        };
        let snapshot = refresh.snapshot_status();
        broadcast_sync_status_heartbeat(&session, &snapshot);
        if state_changed {
            broadcast_sync_status_transition(&session, &snapshot);
        }
    }
}

pub(crate) fn broadcast_sync_status_heartbeat(
    session: &crate::session::Session,
    snapshot: &crate::sync::SyncStatus,
) {
    let status = serde_json::to_value(snapshot).unwrap_or(serde_json::Value::Null);
    let msg = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::SyncPoll { status }),
        ..crate::message::Message::default()
    };
    session.broadcast_ephemeral(&msg);
}

pub(crate) fn broadcast_sync_status_transition(
    session: &crate::session::Session,
    snapshot: &crate::sync::SyncStatus,
) {
    if let Some(transition) = crate::sync::sync_transition(snapshot) {
        let delta = crate::message::Message {
            from: "bpmux".to_string(),
            text: String::new(),
            ts: chrono::Utc::now(),
            message_type: crate::message::MessageType::StateDelta,
            state_delta: Some(crate::message::StateDeltaPayload::Sync {
                transition: crate::message::SyncTransitionWire {
                    status: transition.status.to_string(),
                    why: transition.why,
                    pending_events: transition.pending_events,
                    conflicts_pending: transition.conflicts_pending,
                },
            }),
            ..crate::message::Message::default()
        };
        if let Err(e) = session.broadcast(&delta) {
            crate::diag::log_diag(&format!(
                "run_drain_loop: sync-status state-delta broadcast failed: {e}"
            ));
        }
    }
}

pub(crate) fn drain_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_DRAIN_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    std::time::Duration::from_secs(secs.max(1))
}
