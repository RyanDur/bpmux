use std::sync::Arc;

use sync::SyncOrchestrator;

pub trait SyncStatusSource: Send + Sync {
    fn status(&self) -> source_sync::SyncStatus;
}

impl SyncStatusSource for SyncOrchestrator {
    fn status(&self) -> source_sync::SyncStatus {
        self.snapshot_status()
    }
}

pub async fn run_sync_status_heartbeat(
    source: Arc<dyn SyncStatusSource>,
    hub: Arc<hub::Hub>,
    interval: std::time::Duration,
    connected: super::ConnectedProbe,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        if !connected() {
            continue;
        }
        let snapshot = source.status();
        broadcast_sync_status_heartbeat(&hub, &snapshot);
    }
}

pub fn broadcast_sync_status_heartbeat(hub: &hub::Hub, snapshot: &source_sync::SyncStatus) {
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

pub fn sync_status_heartbeat_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_SYNC_HEARTBEAT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    std::time::Duration::from_secs(secs.max(1))
}
