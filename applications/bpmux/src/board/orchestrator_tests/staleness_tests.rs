#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState, StoryType};
use crate::board::events::{BoardEvent, EventMeta};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::peer::PeerId;
use crate::board::store::BoardStore;

mod staleness {
    use super::*;
    use crate::board::orchestrator::is_stale;

    #[test]
    fn no_ingest_yet_is_not_stale() {
        let now = Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap();
        assert!(
            !is_stale(None, 180, now),
            "fresh session is connecting, not stale"
        );
    }

    #[test]
    fn recent_ingest_is_not_stale() {
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        let now = last + chrono::Duration::seconds(60);
        assert!(
            !is_stale(Some(last), 180, now),
            "60s since last ingest under 180s threshold = fresh"
        );
    }

    #[test]
    fn ingest_older_than_threshold_is_stale() {
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        let now = last + chrono::Duration::seconds(181);
        assert!(
            is_stale(Some(last), 180, now),
            "181s since last ingest exceeds 180s threshold = stale"
        );
    }

    #[test]
    fn ingest_at_exact_threshold_is_not_stale() {
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        let now = last + chrono::Duration::seconds(180);
        assert!(
            !is_stale(Some(last), 180, now),
            "exactly threshold seconds is the boundary, not yet stale"
        );
    }

    #[test]
    fn future_ingest_timestamp_is_not_stale() {
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        let now = last - chrono::Duration::seconds(10);
        assert!(
            !is_stale(Some(last), 180, now),
            "clock skew (last in future) doesn't trip stale"
        );
    }

    #[tokio::test]
    async fn snapshot_status_default_threshold_keeps_fresh_ingest_not_stale() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        orch.ingest_once().await.unwrap();
        let snap = orch.snapshot_status();
        assert!(snap.last_ingest_at.is_some(), "ingest happened");
        assert!(
            !snap.stale,
            "ingest milliseconds ago is well under default 180s threshold: {snap:?}"
        );
        assert!(!snap.startup_fail, "successful ingest is not startup-fail");
    }

    #[test]
    fn startup_fail_when_no_ingest_yet_and_last_error_set() {
        use crate::board::orchestrator::is_startup_fail;
        assert!(is_startup_fail(None, Some("rate-limited")));
    }

    #[test]
    fn no_startup_fail_when_ingest_succeeded_even_with_recent_error() {
        use crate::board::orchestrator::is_startup_fail;
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        assert!(!is_startup_fail(
            Some(last),
            Some("post-success transient error")
        ));
    }

    #[test]
    fn no_startup_fail_when_no_ingest_yet_but_no_error_recorded() {
        use crate::board::orchestrator::is_startup_fail;
        assert!(
            !is_startup_fail(None, None),
            "fresh-session-pre-first-tick is not startup-fail until an error has been recorded"
        );
    }

    #[tokio::test]
    async fn snapshot_status_surfaces_startup_fail_when_ingest_fails_with_no_prior_success() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.set_ingest_failures(true);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        let _ = orch.ingest_once().await;
        let snap = orch.snapshot_status();
        assert!(snap.last_ingest_at.is_none(), "no successful ingest yet");
        assert!(snap.last_error.is_some(), "ingest error recorded");
        assert!(
            snap.startup_fail,
            "no-ingest + last_error → startup_fail surfaces in snapshot: {snap:?}"
        );
    }
}
