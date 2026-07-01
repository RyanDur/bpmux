#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::events::{EventMeta, StoryEvent};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{BoardState, Story, StoryId};

mod outbox_coalesce_1339_step_2e {
    use super::*;

    fn extract_label(event: &StoryEvent) -> Option<(&'static str, &str)> {
        match event {
            StoryEvent::TagAdded { label, .. } => Some(("A", label.as_str())),
            StoryEvent::TagRemoved { label, .. } => Some(("R", label.as_str())),
            _ => None,
        }
    }

    fn queue_labels(orch: &SyncOrchestrator<InMemorySourceAdapter>) -> Vec<(String, String)> {
        orch.snapshot_pending_for_test()
            .iter()
            .filter_map(|q| extract_label(&q.event).map(|(k, l)| (k.to_string(), l.to_string())))
            .collect()
    }

    #[tokio::test]
    async fn rapid_drag_sequence_coalesces_to_first_remove_plus_last_add_per_class() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let mut seed = seed_story(7011, BoardState::Icebox);
        seed.raw_labels = vec!["iteration-Pegasus.300".to_string()];
        store.seed_all(vec![seed]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7011),
            label: "iteration-Pegasus.150".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagRemoved {
            meta: meta(),
            id: StoryId::new(7011),
            label: "iteration-Pegasus.300".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7011),
            label: "iteration-Pegasus.75".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagRemoved {
            meta: meta(),
            id: StoryId::new(7011),
            label: "iteration-Pegasus.150".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7011),
            label: "iteration-Pegasus.50".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagRemoved {
            meta: meta(),
            id: StoryId::new(7011),
            label: "iteration-Pegasus.75".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels,
            vec![
                ("R".to_string(), "iteration-Pegasus.300".to_string()),
                ("A".to_string(), "iteration-Pegasus.50".to_string()),
            ],
            "rapid drag should coalesce to first-remove + last-add: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_per_class_keeps_distinct_iteration_namespaces_apart() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(7012, BoardState::Icebox)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7012),
            label: "iteration-Pegasus.100".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7012),
            label: "iteration-order.50".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7012),
            label: "iteration-Pegasus.50".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels,
            vec![
                ("A".to_string(), "iteration-order.50".to_string()),
                ("A".to_string(), "iteration-Pegasus.50".to_string()),
            ],
            "different prefix-classes coalesce independently: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_does_not_collapse_across_stories() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![
            seed_story(7013, BoardState::Icebox),
            seed_story(7014, BoardState::Icebox),
        ]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7013),
            label: "iteration-Pegasus.100".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7014),
            label: "iteration-Pegasus.100".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7013),
            label: "iteration-Pegasus.50".to_string(),
        })
        .unwrap();

        let pending = orch.snapshot_pending_for_test();
        let by_story: Vec<(u64, String)> = pending
            .iter()
            .filter_map(|q| match &q.event {
                StoryEvent::TagAdded { id, label, .. } => Some((id.value(), label.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            by_story,
            vec![
                (7014, "iteration-Pegasus.100".to_string()),
                (7013, "iteration-Pegasus.50".to_string()),
            ],
            "cross-story events stay independent; only intra-story coalesces: {by_story:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_does_not_touch_non_iteration_labels() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(7015, BoardState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7015),
            label: "blocked".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagRemoved {
            meta: meta(),
            id: StoryId::new(7015),
            label: "ready".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7015),
            label: "in progress".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels.len(),
            3,
            "non-iteration labels do not coalesce: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_handles_add_only_sequence_keeping_last_add() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(7016, BoardState::Icebox)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7016),
            label: "iteration-Pegasus.200".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7016),
            label: "iteration-Pegasus.150".to_string(),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(7016),
            label: "iteration-Pegasus.100".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels,
            vec![("A".to_string(), "iteration-Pegasus.100".to_string())],
            "Add-only coalesces to last Add: {labels:?}"
        );
    }
}
