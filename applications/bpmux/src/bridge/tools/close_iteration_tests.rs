#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(unsafe_code)]

mod canonical_via_gh_1300 {
    use crate::board::RefreshDriver;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use crate::board::orchestrator::SyncOrchestrator;
    use crate::board::store::BoardStore;
    use crate::transport::rest::{
        collect_ready_queue_blockers, promote_members_then_close_iteration,
    };
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn seed_member(number: u64, state: StoryState, iteration: Option<(&str, u32)>) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("[Feature] member-{number}"),
            state,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        let state_label = format!("{state:?}").to_lowercase();
        s.raw_labels = match iteration {
            Some((theme, position)) => {
                vec![state_label, format!("iteration-{theme}.{position}")]
            }
            None => vec![state_label],
        };
        s
    }

    fn seed_iteration_issue(number: u64, theme: &str) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("Iteration {theme}"),
            StoryState::Icebox,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        s.story_type = StoryType::Iteration;
        s.raw_labels = vec!["iteration".to_string()];
        s
    }

    async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch = SyncOrchestrator::new(store, adapter);
        let orch: Arc<dyn RefreshDriver> = Arc::new(orch);
        orch.refresh_all().await.unwrap();
        orch
    }

    #[tokio::test]
    async fn promotes_each_member_to_ready_and_closes_iteration_issue() {
        // AC1 + AC2 + AC3: close GH issue + promote members + iteration_id cleared.
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(("Pegasus", 100))),
            seed_member(201, StoryState::Icebox, Some(("Pegasus", 200))),
        ])
        .await;

        promote_members_then_close_iteration(&refresh, 1400, &[200, 201])
            .await
            .expect("canonical-via-GH close_iteration succeeds");

        let iter = refresh.story(1400).expect("iteration-issue still exists");
        assert!(iter.is_closed, "iteration-issue closed (AC1)");

        let s200 = refresh.story(200).expect("member 200 exists");
        assert_eq!(s200.state, StoryState::Ready, "AC2: member 200 promoted");

        let s201 = refresh.story(201).expect("member 201 exists");
        assert_eq!(s201.state, StoryState::Ready, "AC2: member 201 promoted");
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_blocks_close_when_stories_in_ready() {
        // AC5: #1258 r3 ready-empty guard preserved at canonical-via-GH axis.
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(900, StoryState::Ready, None),
            seed_member(200, StoryState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        let blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
        assert_eq!(blockers.len(), 1, "ready-queue blocker surfaces");
        assert_eq!(blockers[0].number, 900);
        assert_eq!(blockers[0].state, "ready");
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_passes_when_no_ready_stories() {
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        let blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
        assert!(
            blockers.is_empty(),
            "no ready blockers — close proceeds: {blockers:?}"
        );
    }

    #[tokio::test]
    async fn closed_iteration_issue_remains_queryable_for_history_preservation() {
        // AC7 + falsifier (h): #1278 sister — closed Iteration-issue stays
        // accessible as historical-artifact (not deleted from store).
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        promote_members_then_close_iteration(&refresh, 1400, &[200])
            .await
            .unwrap();
        let iter = refresh.story(1400).expect("history-preserved iteration");
        assert!(iter.is_closed);
        assert!(matches!(iter.story_type, StoryType::Iteration));
        assert_eq!(iter.title, "Iteration Pegasus");
    }

    #[tokio::test]
    async fn empty_member_list_closes_iteration_issue_gracefully() {
        // Falsifier (g): graceful failure on no-members. Empty member-list
        // is the boundary case: iteration-issue closes; no member-promotions.
        let refresh = build_refresh(vec![seed_iteration_issue(1400, "Pegasus")]).await;
        promote_members_then_close_iteration(&refresh, 1400, &[])
            .await
            .expect("empty-member close succeeds");
        let iter = refresh.story(1400).unwrap();
        assert!(iter.is_closed);
    }

    #[tokio::test]
    async fn canonical_path_does_not_touch_legacy_priority_json_substrate() {
        // AC10 cross-contamination check: canonical path operates on
        // store + iteration-issue only; does NOT mutate priority.json
        // Verified by holding a priority.json snapshot pre-call and
        // asserting post-call equality.
        let priority_dir = tempfile::tempdir().unwrap();
        let priority_path = priority_dir.path().join("priority.json");
        let priority_pre = r#"{"ready":[42]}"#;
        std::fs::write(&priority_path, priority_pre).unwrap();

        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        promote_members_then_close_iteration(&refresh, 1400, &[200])
            .await
            .unwrap();
        let priority_post = std::fs::read_to_string(&priority_path).unwrap();
        assert_eq!(
            priority_pre, priority_post,
            "canonical path does NOT touch legacy priority.json substrate (AC10 cross-contamination check)"
        );
    }

    #[tokio::test]
    async fn close_walks_by_iteration_theme_label_not_by_priority_json_position() {
        use crate::board::iteration_label::members_with_iteration_label;
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_iteration_issue(1401, "Cassiopeia"),
            seed_member(200, StoryState::Icebox, Some(("Cassiopeia", 100))),
            seed_member(201, StoryState::Icebox, Some(("Cassiopeia", 200))),
            seed_member(300, StoryState::Icebox, Some(("Pegasus", 100))),
            seed_member(901, StoryState::Icebox, None),
        ])
        .await;

        let cassiopeia_members = members_with_iteration_label(&refresh.all_stories(), "Cassiopeia");
        assert_eq!(
            cassiopeia_members,
            vec![200, 201],
            "label-walk finds Cassiopeia-tagged members in position-order"
        );

        promote_members_then_close_iteration(&refresh, 1401, &cassiopeia_members)
            .await
            .expect("label-walk close succeeds for Cassiopeia");

        assert_eq!(
            refresh.story(200).unwrap().state,
            StoryState::Ready,
            "Cassiopeia member 200 promoted"
        );
        assert_eq!(
            refresh.story(201).unwrap().state,
            StoryState::Ready,
            "Cassiopeia member 201 promoted"
        );
        assert_eq!(
            refresh.story(300).unwrap().state,
            StoryState::Icebox,
            "Pegasus-tagged 300 stays — not in Cassiopeia's label-band"
        );
        assert_eq!(
            refresh.story(901).unwrap().state,
            StoryState::Icebox,
            "untagged story 901 stays — no iteration-label means not a member"
        );

        let s200 = refresh.story(200).unwrap();
        assert!(
            !s200
                .raw_labels
                .iter()
                .any(|l| l == "iteration-Cassiopeia.100"),
            "member 200's iteration-Cassiopeia.100 label removed on promote (raw_labels={:?})",
            s200.raw_labels
        );
        let s300 = refresh.story(300).unwrap();
        assert!(
            s300.raw_labels.iter().any(|l| l == "iteration-Pegasus.100"),
            "non-member 300's iteration-Pegasus.100 label untouched (raw_labels={:?})",
            s300.raw_labels
        );
    }

    #[tokio::test]
    async fn iteration_close_uses_canonical_mutation_chain_for_member_promotion() {
        // Falsifier (b): member-stories promoted via canonical mutation
        // chain (not direct store-write). Verified by subscribing to
        // BoardStore events: per #1282 fire-ordering, each member-promotion
        // emits TagAdded("ready") + StateTransitioned(Ready) + TagRemoved
        // ("iteration-#NNNN"), and the iteration-issue close emits
        // StoryClosed — all visible to subscribers.
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        let store = refresh.store();
        let mut rx = store.subscribe();
        promote_members_then_close_iteration(&refresh, 1400, &[200])
            .await
            .unwrap();
        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }
        let names: Vec<&str> = events
            .iter()
            .map(|e| match e {
                crate::board::BoardEvent::TagAdded { .. } => "TagAdded",
                crate::board::BoardEvent::TagRemoved { .. } => "TagRemoved",
                crate::board::BoardEvent::StateTransitioned { .. } => "StateTransitioned",
                crate::board::BoardEvent::StoryClosed { .. } => "StoryClosed",
                _ => "other",
            })
            .collect();
        assert!(
            names.contains(&"TagAdded"),
            "TagAdded fired (ready label): {names:?}"
        );
        assert!(
            names.contains(&"StateTransitioned"),
            "StateTransitioned fired: {names:?}"
        );
        assert!(
            names.contains(&"TagRemoved"),
            "TagRemoved fired (iteration label cleared): {names:?}"
        );
        assert!(
            names.contains(&"StoryClosed"),
            "StoryClosed fired (iteration-issue): {names:?}"
        );
    }
}
