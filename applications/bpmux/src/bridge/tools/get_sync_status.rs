use std::sync::Arc;

use crate::board::RefreshDriver;

pub(in crate::bridge) async fn handle_get_sync_status(
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let status = match refresh {
        Some(r) => r.snapshot_status(),
        None => crate::board::orchestrator::SyncStatus::default(),
    };
    serde_json::to_string_pretty(&status).map_err(|e| format!("serialize: {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::board::events::BoardEvent;
    use crate::board::orchestrator::{BoxedFuture, SyncStatus};
    use crate::board::sync::SyncError;
    use chrono::{TimeZone, Utc};

    struct StubDriver(SyncStatus);

    impl RefreshDriver for StubDriver {
        fn refresh_story(
            &self,
            _number: u64,
        ) -> BoxedFuture<'_, Result<Option<crate::board::Story>, SyncError>> {
            Box::pin(async { Ok(None) })
        }
        fn refresh_all(&self) -> BoxedFuture<'_, Result<(), SyncError>> {
            Box::pin(async { Ok(()) })
        }
        fn mutate_sync(&self, _event: BoardEvent) -> Result<(), SyncError> {
            Ok(())
        }
        fn snapshot_status(&self) -> SyncStatus {
            self.0.clone()
        }
    }

    #[tokio::test]
    async fn returns_default_status_when_refresh_absent() {
        let out = handle_get_sync_status(None).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("last_ingest_at").is_some());
        assert!(v.get("last_push_at").is_some());
        assert_eq!(v["pending_events"], 0);
        assert!(v.get("last_error").is_some());
    }

    #[tokio::test]
    async fn surfaces_driver_state_when_present() {
        let ingest_at = Utc.with_ymd_and_hms(2026, 4, 24, 18, 0, 0).unwrap();
        let status = SyncStatus {
            last_ingest_at: Some(ingest_at),
            last_push_at: None,
            pending_events: 3,
            last_error: Some("source unreachable".into()),
        };
        let driver: Arc<dyn RefreshDriver> = Arc::new(StubDriver(status));
        let out = handle_get_sync_status(Some(&driver)).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["pending_events"], 3);
        assert_eq!(v["last_error"], "source unreachable");
        assert_eq!(v["last_ingest_at"], "2026-04-24T18:00:00Z");
        assert!(v["last_push_at"].is_null());
    }
}
