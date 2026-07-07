use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::seed_story;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::{CastVerdict, StoryId, Verdict, Work};
use crate::store::SeededStore;
use crate::sync::{RefreshDriver, SyncOrchestrator, backfill_shared_history};

fn ready_verdict(hour: u32, actor: &str) -> CastVerdict {
    CastVerdict {
        verdict: Verdict::Succeed {
            step: "ready".into(),
        },
        ts: Utc.with_ymd_and_hms(2026, 7, 7, hour, 0, 0).unwrap(),
        actor: actor.into(),
        payload: Some("pickup".into()),
    }
}

#[tokio::test]
async fn a_cache_fill_leaves_the_shared_grain_empty_but_the_boot_sweep_fills_it_from_durable_verdicts()
 {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(11, Work::InProgress)]);
    adapter.seed_verdicts(11, vec![ready_verdict(9, "dev")]);
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator.ingest_once().await.unwrap();

    orchestrator.fetch_verdicts(11).await.unwrap();
    let after_cache_fill = handle
        .select()
        .select(|s| s.shared_history.records().to_vec());
    assert!(
        after_cache_fill.is_empty(),
        "a VerdictsFetched cache fill is not a gate observation — the shared grain stays empty"
    );

    backfill_shared_history(orch_dyn.as_ref(), |action| handle.dispatch(action)).await;
    let after_backfill = handle
        .select()
        .select(|s| s.shared_history.records().to_vec());
    assert_eq!(
        after_backfill.len(),
        1,
        "the deliberate backfill flow rebuilds the shared grain from durable verdicts"
    );
    assert_eq!(after_backfill[0].story, StoryId::new(11));
    assert_eq!(
        after_backfill[0].actor, "dev",
        "the record carries the verdict's durable GH actor, never a local synthetic"
    );
    assert_eq!(
        after_backfill[0].ts,
        Utc.with_ymd_and_hms(2026, 7, 7, 9, 0, 0).unwrap(),
        "the record carries the durable GH timestamp — the anti-confabulation guarantee rides the fetched verdict"
    );
}

#[tokio::test]
async fn the_boot_sweep_replaces_each_storys_records_per_story() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![
        seed_story(11, Work::InProgress),
        seed_story(12, Work::InProgress),
    ]);
    adapter.seed_verdicts(11, vec![ready_verdict(9, "dev")]);
    adapter.seed_verdicts(12, vec![ready_verdict(10, "owner")]);
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator.ingest_once().await.unwrap();

    backfill_shared_history(orch_dyn.as_ref(), |action| handle.dispatch(action)).await;
    let records = handle
        .select()
        .select(|s| s.shared_history.records().to_vec());
    assert_eq!(records.len(), 2, "one durable record per swept story");
    assert!(
        records
            .iter()
            .any(|r| r.story == StoryId::new(11) && r.actor == "dev")
    );
    assert!(
        records
            .iter()
            .any(|r| r.story == StoryId::new(12) && r.actor == "owner")
    );
}
