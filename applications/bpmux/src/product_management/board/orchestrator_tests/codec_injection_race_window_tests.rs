#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::events::{BoardEvent, EventMeta};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{Story, StoryId, StoryState, StoryType};

mod codec_injection_substrate {
    use super::*;
    use crate::product_management::board::label_codec::LabelCodec;
    use crate::product_management::iteration::label::IterationLabelParts;
    use std::sync::Arc;

    struct StubLabelCodec;

    impl LabelCodec for StubLabelCodec {
        fn decode_ready_order(&self, label: &str) -> Option<u32> {
            label.strip_prefix("rdy:").and_then(|s| s.parse().ok())
        }

        fn encode_ready_order(&self, position: u32) -> String {
            format!("rdy:{position}")
        }

        fn decode_iteration(&self, label: &str) -> Option<IterationLabelParts> {
            let rest = label.strip_prefix("STUB|")?;
            let (name, pos) = rest.split_once(':')?;
            let position: u32 = pos.parse().ok()?;
            Some(IterationLabelParts {
                name: name.to_string(),
                position,
            })
        }

        fn is_iteration_label_loose(&self, label: &str) -> bool {
            label.starts_with("STUB|") && label.contains(':')
        }

        fn encode_iteration(&self, name: &str, position: u32) -> String {
            format!("STUB|{name}:{position}")
        }

        fn iteration_label_class(&self, label: &str) -> Option<String> {
            let rest = label.strip_prefix("STUB|")?;
            let (name, _pos) = rest.split_once(':')?;
            Some(format!("STUB|{name}"))
        }

        fn encode_parent_epic(&self, issue_number: u64) -> String {
            format!("epic#{issue_number}")
        }
    }

    fn meta() -> EventMeta {
        EventMeta {
            actor: "stub".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).unwrap(),
            peer_id: PeerId::local(),
        }
    }

    fn seed_story(id: u64) -> Story {
        Story::new_minimal(
            StoryId::new(id),
            format!("story-{id}"),
            StoryState::InProgress,
            Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).unwrap(),
            "pm".to_string(),
        )
    }

    fn queue_labels(orch: &SyncOrchestrator<InMemorySourceAdapter>) -> Vec<(String, String)> {
        orch.snapshot_pending_for_test()
            .iter()
            .map(|q| {
                let (k, l) = match &q.event {
                    BoardEvent::TagAdded { label, .. } => ("A", label.clone()),
                    BoardEvent::TagRemoved { label, .. } => ("R", label.clone()),
                    _ => ("?", String::new()),
                };
                (k.to_string(), l)
            })
            .collect()
    }

    #[tokio::test]
    async fn coalescer_uses_injected_codec_classification() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(8801)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter))
            .with_codec(Arc::new(StubLabelCodec));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(8801),
            label: "STUB|Andromeda:100".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(8801),
            label: "STUB|Andromeda:200".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels,
            vec![("A".to_string(), "STUB|Andromeda:200".to_string())],
            "stub codec's iteration_label_class drove coalesce: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalescer_keeps_distinct_stub_iteration_namespaces_apart() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(8802)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter))
            .with_codec(Arc::new(StubLabelCodec));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(8802),
            label: "STUB|Andromeda:100".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(8802),
            label: "STUB|Pegasus:50".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels.len(),
            2,
            "distinct iteration-class labels preserved by stub codec: {labels:?}"
        );
    }
}

#[cfg(test)]
mod race_window_barrier {

    use crate::product_management::board::RaceWindowBarrier;
    use std::sync::Arc;

    #[tokio::test]
    async fn target_zero_arrive_is_pass_through() {
        let barrier = RaceWindowBarrier::new(0);
        barrier.arrive().await;
    }

    #[tokio::test]
    async fn target_zero_complete_is_no_op() {
        let barrier = RaceWindowBarrier::new(0);
        barrier.complete();
    }

    #[tokio::test]
    async fn complete_is_idempotent_under_target_two_rendezvous() {
        let barrier = Arc::new(RaceWindowBarrier::new(2));
        let a = Arc::clone(&barrier);
        let b = Arc::clone(&barrier);
        let task_a = tokio::spawn(async move {
            a.arrive().await;
            a.complete();
            a.complete();
        });
        let task_b = tokio::spawn(async move {
            b.arrive().await;
            b.complete();
        });
        task_a.await.expect("task a");
        task_b.await.expect("task b");
    }
}
