#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, SeededStore};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{Story, StoryId, Work};

mod outbox_coalesce_1339_step_2e {
    use super::*;

    fn extract_label(event: &StoryEvent) -> Option<(&'static str, &str)> {
        match &event.kind {
            StoryEventKind::TagAdded { label } => Some(("A", label.as_str())),
            StoryEventKind::TagRemoved { label } => Some(("R", label.as_str())),
            _ => None,
        }
    }

    fn queue_labels(handle: &SeededStore) -> Vec<(String, String)> {
        handle
            .pending_writes()
            .iter()
            .filter_map(|q| extract_label(&q.event).map(|(k, l)| (k.to_string(), l.to_string())))
            .collect()
    }

    #[tokio::test]
    async fn rapid_drag_sequence_coalesces_to_first_remove_plus_last_add_per_class() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let seed = seed_story(7011, Work::NotStarted);
        let orch = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let orch_dyn: Arc<SyncOrchestrator> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed])));

        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7011),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.150".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7011),
            StoryEventKind::TagRemoved {
                label: "iteration-Pegasus.300".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7011),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.75".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7011),
            StoryEventKind::TagRemoved {
                label: "iteration-Pegasus.150".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7011),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.50".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7011),
            StoryEventKind::TagRemoved {
                label: "iteration-Pegasus.75".to_string(),
            },
        ))
        .unwrap();

        let labels = queue_labels(&handle);
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
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let orch_dyn: Arc<SyncOrchestrator> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            7012,
            Work::NotStarted,
        )])));

        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7012),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.100".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7012),
            StoryEventKind::TagAdded {
                label: "iteration-order.50".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7012),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.50".to_string(),
            },
        ))
        .unwrap();

        let labels = queue_labels(&handle);
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
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let orch_dyn: Arc<SyncOrchestrator> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![
            seed_story(7013, Work::NotStarted),
            seed_story(7014, Work::NotStarted),
        ])));

        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7013),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.100".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7014),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.100".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7013),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.50".to_string(),
            },
        ))
        .unwrap();

        let pending = handle.pending_writes();
        let by_story: Vec<(u64, String)> = pending
            .iter()
            .filter_map(|q| match &q.event.kind {
                StoryEventKind::TagAdded { label } => Some((q.event.id.value(), label.clone())),
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
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let orch_dyn: Arc<SyncOrchestrator> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            7015,
            Work::NotStarted,
        )])));

        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7015),
            StoryEventKind::TagAdded {
                label: "blocked".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7015),
            StoryEventKind::TagRemoved {
                label: "ready".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7015),
            StoryEventKind::TagAdded {
                label: "in progress".to_string(),
            },
        ))
        .unwrap();

        let labels = queue_labels(&handle);
        assert_eq!(
            labels.len(),
            3,
            "non-iteration labels do not coalesce: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_handles_add_only_sequence_keeping_last_add() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let orch_dyn: Arc<SyncOrchestrator> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            7016,
            Work::NotStarted,
        )])));

        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7016),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.200".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7016),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.150".to_string(),
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(7016),
            StoryEventKind::TagAdded {
                label: "iteration-Pegasus.100".to_string(),
            },
        ))
        .unwrap();

        let labels = queue_labels(&handle);
        assert_eq!(
            labels,
            vec![("A".to_string(), "iteration-Pegasus.100".to_string())],
            "Add-only coalesces to last Add: {labels:?}"
        );
    }
}
