#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::transport::protocol::ClientMessage;

#[cfg(test)]
mod reorder_story_icebox_label_substrate_tests_1301 {
    use crate::bridge::tools_stories::reorder_icebox_story_via_labels;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Story, StoryId};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn seed_story(id: u64, labels: Vec<String>) -> Story {
        let mut s = Story::new_minimal(
            StoryId::new(id),
            format!("story-{id}"),
            BoardState::Icebox,
            Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        s.raw_labels = labels;
        s
    }

    async fn build_orchestrator(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
        orch.refresh_all().await.unwrap();
        orch
    }

    async fn reorder(
        orch: &Arc<dyn RefreshDriver>,
        id: u64,
        position: u64,
    ) -> Result<String, String> {
        reorder_icebox_story_via_labels(orch, id, position as usize).map(|_| String::new())
    }

    #[tokio::test]
    async fn icebox_reorder_with_existing_iteration_label_keeps_name_and_repositions() {
        let orch = build_orchestrator(vec![
            seed_story(2001, vec!["iteration-Pegasus.100".into()]),
            seed_story(2002, vec!["iteration-Pegasus.200".into()]),
            seed_story(2003, vec!["iteration-Pegasus.300".into()]),
        ])
        .await;
        reorder(&orch, 2003, 1).await.unwrap();
        let s = orch.story(2003).unwrap();
        assert!(
            s.raw_labels.iter().any(|l| l == "iteration-Pegasus.50"),
            "story 2003 should land at the head with iteration-Pegasus.50 (midpoint of (0, 100)); got {:?}",
            s.raw_labels
        );
        assert!(
            !s.raw_labels.iter().any(|l| l == "iteration-Pegasus.300"),
            "old iteration-Pegasus.300 label should be removed: {:?}",
            s.raw_labels
        );
    }

    #[tokio::test]
    async fn icebox_reorder_loose_story_lands_in_unscheduled_container() {
        let orch = build_orchestrator(vec![
            seed_story(2010, vec!["icebox".into()]),
            seed_story(2011, vec!["iteration-Unscheduled.100".into()]),
            seed_story(2012, vec!["iteration-Unscheduled.200".into()]),
        ])
        .await;
        reorder(&orch, 2010, 1).await.unwrap();
        let s = orch.story(2010).unwrap();
        assert!(
            s.raw_labels.iter().any(|l| l == "iteration-Unscheduled.50"),
            "loose story should join Unscheduled at the head (midpoint of (0, 100)); got {:?}",
            s.raw_labels
        );
    }

    #[tokio::test]
    async fn icebox_reorder_handles_tail_position_with_default_step() {
        let orch = build_orchestrator(vec![
            seed_story(2020, vec!["iteration-Pegasus.100".into()]),
            seed_story(2021, vec!["iteration-Pegasus.200".into()]),
        ])
        .await;
        reorder(&orch, 2020, 99).await.unwrap();
        let s = orch.story(2020).unwrap();
        assert!(
            s.raw_labels.iter().any(|l| l == "iteration-Pegasus.300"),
            "tail position should compute last+step (200+100=300); got {:?}",
            s.raw_labels
        );
    }

    #[tokio::test]
    async fn icebox_reorder_no_op_when_position_resolves_to_same_label() {
        let orch = build_orchestrator(vec![
            seed_story(2030, vec!["iteration-Pegasus.100".into()]),
            seed_story(2031, vec!["iteration-Pegasus.200".into()]),
            seed_story(2032, vec!["iteration-Pegasus.300".into()]),
        ])
        .await;
        let labels_before = orch.story(2031).unwrap().raw_labels.clone();
        reorder(&orch, 2031, 2).await.unwrap();
        let labels_after = orch.story(2031).unwrap().raw_labels.clone();
        assert_eq!(
            labels_before, labels_after,
            "reorder to current position should be a no-op"
        );
    }

    #[tokio::test]
    async fn icebox_reorder_rejects_when_story_not_in_store() {
        let orch = build_orchestrator(vec![]).await;
        let err = reorder(&orch, 9999, 1).await.unwrap_err();
        assert!(
            err.contains("not found"),
            "expected not-found error; got {err}"
        );
    }
}
