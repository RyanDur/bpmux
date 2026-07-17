use std::sync::Arc;

use crate::sync::{BoxedFuture, SyncError, SyncOrchestrator};

pub trait DrainTicker: Send + Sync {
    fn drain(&self) -> BoxedFuture<'_, Result<usize, SyncError>>;
    fn status(&self) -> source_sync::SyncStatus;
}

impl DrainTicker for SyncOrchestrator {
    fn drain(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
        Box::pin(self.drain_pass())
    }

    fn status(&self) -> source_sync::SyncStatus {
        self.snapshot_status()
    }
}

pub(crate) async fn run_drain_loop(
    refresh: Arc<dyn DrainTicker>,
    hub: Arc<crate::hub::Hub>,
    interval: std::time::Duration,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        match refresh.drain().await {
            Ok(0) => {}
            Ok(n) => {
                diag::log_diag(&format!("bridge: background drain pushed {n} event(s)"));
            }
            Err(e) => {
                diag::log_diag(&format!("bridge: background drain: {e}"));
            }
        }
        let snapshot = refresh.status();
        broadcast_sync_status_heartbeat(&hub, &snapshot);
    }
}

pub(crate) fn broadcast_sync_status_heartbeat(
    hub: &crate::hub::Hub,
    snapshot: &source_sync::SyncStatus,
) {
    let status = serde_json::to_value(snapshot).unwrap_or(serde_json::Value::Null);
    let msg = message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::SyncPoll { status }),
        ..message::Message::default()
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
