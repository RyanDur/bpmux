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
    use crate::transport::rest::{close_iteration_canonical, collect_ready_queue_blockers};
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
    async fn close_promotes_nothing_and_does_not_error_on_canonical_member_1556() {
        let mut canonical_member = Story::new_minimal(
            200,
            "[Feature] canonical-member".to_string(),
            StoryState::Icebox,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        canonical_member.raw_labels = vec![
            "icebox".to_string(),
            "parent-iteration-#1400.100".to_string(),
        ];
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            canonical_member,
        ])
        .await;

        close_iteration_canonical(&refresh, 1400)
            .await
            .expect("close succeeds without erroring on a canonically-attributed member");

        let iter = refresh.story(1400).expect("iteration-issue still exists");
        assert!(iter.is_closed, "iteration closed — close is terminal");
        let member = refresh.story(200).expect("member 200 exists");
        assert_eq!(
            member.state,
            StoryState::Icebox,
            "#1556: close promotes NOTHING — the member stays in its state, not moved to ready"
        );
    }

    #[tokio::test]
    async fn closing_the_current_iteration_clears_its_designation_1540() {
        let mut current = seed_iteration_issue(1400, "Pegasus");
        current.raw_labels.push("current-iteration".to_string());
        current.is_current = true;
        let refresh = build_refresh(vec![current]).await;
        assert!(
            refresh.story(1400).expect("iteration exists").is_current,
            "precondition: iteration #1400 is the current designation"
        );

        close_iteration_canonical(&refresh, 1400)
            .await
            .expect("canonical-via-GH close_iteration succeeds");

        let iter = refresh.story(1400).expect("iteration-issue still exists");
        assert!(iter.is_closed, "iteration closed");
        assert!(
            !iter.is_current,
            "#1540 AC1: closing the current iteration clears its designation — NOW falls to honest-empty, never auto-jumps"
        );
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_blocks_close_when_stories_in_ready() {
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
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        close_iteration_canonical(&refresh, 1400).await.unwrap();
        let iter = refresh.story(1400).expect("history-preserved iteration");
        assert!(iter.is_closed);
        assert!(matches!(iter.story_type, StoryType::Iteration));
        assert_eq!(iter.title, "Iteration Pegasus");
    }

    #[tokio::test]
    async fn empty_member_list_closes_iteration_issue_gracefully() {
        let refresh = build_refresh(vec![seed_iteration_issue(1400, "Pegasus")]).await;
        close_iteration_canonical(&refresh, 1400)
            .await
            .expect("empty-member close succeeds");
        let iter = refresh.story(1400).unwrap();
        assert!(iter.is_closed);
    }

    #[tokio::test]
    async fn close_fires_storyclosed_without_member_promotion_mutations_1556() {
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        let store = refresh.store();
        let mut rx = store.subscribe();
        close_iteration_canonical(&refresh, 1400).await.unwrap();
        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }
        let member_promotion_touch = events.iter().any(|e| {
            matches!(
                e,
                crate::board::BoardEvent::TagAdded { number, .. }
                    | crate::board::BoardEvent::StateTransitioned { number, .. }
                    if *number == 200
            )
        });
        assert!(
            !member_promotion_touch,
            "#1556: close promotes nothing — no member TagAdded/StateTransitioned: {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, crate::board::BoardEvent::StoryClosed { .. })),
            "StoryClosed still fires for the iteration-issue: {events:?}"
        );
    }
}
