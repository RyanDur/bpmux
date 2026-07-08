use std::sync::Arc;

pub(crate) async fn run_drain_loop(
    refresh: Arc<dyn crate::sync::RefreshDriver>,
    hub: Arc<crate::hub::Hub>,
    interval: std::time::Duration,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        match refresh.drain_pending().await {
            Ok(0) => {}
            Ok(n) => {
                crate::diag::log_diag(&format!("bridge: background drain pushed {n} event(s)"));
            }
            Err(e) => {
                crate::diag::log_diag(&format!("bridge: background drain: {e}"));
            }
        }
        let snapshot = refresh.snapshot_status();
        broadcast_sync_status_heartbeat(&hub, &snapshot);
    }
}

pub(crate) fn broadcast_sync_status_heartbeat(
    hub: &crate::hub::Hub,
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
    hub.broadcast_ephemeral(&msg);
}

pub(crate) fn drain_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_DRAIN_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    std::time::Duration::from_secs(secs.max(1))
}
