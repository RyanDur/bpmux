#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(unsafe_code)]

mod canonical_via_gh_1300 {
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::adapters::github::decode_iteration_from_story;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::board::orchestrator::SyncOrchestrator;
    use crate::product_management::board::store::BoardStore;
    use crate::product_management::iteration::{Iteration, IterationAction, IterationId};
    use crate::product_management::story::{BoardState, Story, StoryId};
    use crate::store::{AppAction, Dispatch, Select};
    use crate::transport::rest::{close_iteration_canonical, collect_ready_queue_blockers};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;
    use tokio::sync::mpsc::UnboundedReceiver;

    fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
        (Dispatch::new(tx), rx)
    }

    fn closed_iteration(rx: &mut UnboundedReceiver<AppAction>, id: u64) -> Iteration {
        let mut found = None;
        while let Ok(action) = rx.try_recv() {
            if let AppAction::Iteration(IterationAction::Upserted(iterations)) = action
                && let Some(it) = iterations
                    .into_iter()
                    .find(|it| it.id == IterationId::new(id))
            {
                found = Some(it);
            }
        }
        found.expect("the closed iteration is dispatched onto the slice")
    }

    fn seed_member(id: u64, state: BoardState, iteration: Option<(&str, u32)>) -> Story {
        let mut s = Story::new_minimal(
            StoryId::new(id),
            format!("[Feature] member-{id}"),
            state,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        let state_label = format!("{state:?}").to_lowercase();
        s.raw_labels = match iteration {
            Some((name, position)) => {
                vec![state_label, format!("iteration-{name}.{position}")]
            }
            None => vec![state_label],
        };
        s
    }

    fn seed_iteration_issue(id: u64, name: &str) -> Story {
        let mut s = Story::new_minimal(
            StoryId::new(id),
            format!("Iteration {name}"),
            BoardState::Icebox,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        s.raw_labels = vec!["iteration".to_string()];
        s
    }

    async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories.clone());
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
        orch.refresh_all().await.unwrap();
        let iterations: Vec<Iteration> = stories
            .iter()
            .filter(|s| {
                s.raw_labels
                    .iter()
                    .any(|l| l.eq_ignore_ascii_case("iteration"))
            })
            .map(decode_iteration_from_story)
            .collect();
        orch.bind_select(Select::seeded_for_board(
            &orch.store(),
            vec![],
            iterations,
            vec![],
        ));
        orch
    }

    #[tokio::test]
    async fn close_promotes_nothing_and_does_not_error_on_canonical_member_1556() {
        let mut canonical_member = Story::new_minimal(
            StoryId::new(200),
            "[Feature] canonical-member".to_string(),
            BoardState::Icebox,
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
        let (dispatch, mut rx) = dispatch_channel();

        close_iteration_canonical(&refresh, &dispatch, 1400)
            .await
            .expect("close succeeds without erroring on a canonically-attributed member");

        assert!(
            closed_iteration(&mut rx, 1400).complete,
            "iteration closed — close is terminal (dispatched onto the slice as complete)"
        );
        let member = refresh.story(200).expect("member 200 exists");
        assert_eq!(
            member.state,
            BoardState::Icebox,
            "#1556: close promotes NOTHING — the member stays in its state, not moved to ready"
        );
    }

    #[tokio::test]
    async fn closing_the_current_iteration_clears_its_designation_1540() {
        let mut current = seed_iteration_issue(1400, "Pegasus");
        current.raw_labels.push("current-iteration".to_string());
        let refresh = build_refresh(vec![current]).await;
        assert!(
            refresh
                .iteration(IterationId::new(1400))
                .expect("iteration exists")
                .is_current,
            "precondition: iteration #1400 is the current designation"
        );
        let (dispatch, mut rx) = dispatch_channel();

        close_iteration_canonical(&refresh, &dispatch, 1400)
            .await
            .expect("canonical-via-GH close_iteration succeeds");

        let closed = closed_iteration(&mut rx, 1400);
        assert!(closed.complete, "iteration closed");
        assert!(
            !closed.is_current,
            "#1540 AC1: closing the current iteration clears its designation — NOW falls to honest-empty, never auto-jumps"
        );
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_blocks_close_when_stories_in_ready() {
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(900, BoardState::Ready, None),
            seed_member(200, BoardState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        let blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
        assert_eq!(blockers.len(), 1, "ready-queue blocker surfaces");
        assert_eq!(blockers[0].id, 900);
        assert_eq!(blockers[0].state, "ready");
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_passes_when_no_ready_stories() {
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, BoardState::Icebox, Some(("Pegasus", 100))),
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
            seed_member(200, BoardState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        let (dispatch, mut rx) = dispatch_channel();
        close_iteration_canonical(&refresh, &dispatch, 1400)
            .await
            .unwrap();
        let closed = closed_iteration(&mut rx, 1400);
        assert!(
            closed.complete,
            "the iteration is preserved as a complete iteration"
        );
        assert_eq!(
            closed.title, "Iteration Pegasus",
            "the closed iteration keeps its identity/title for history preservation"
        );
    }

    #[tokio::test]
    async fn empty_member_list_closes_iteration_issue_gracefully() {
        let refresh = build_refresh(vec![seed_iteration_issue(1400, "Pegasus")]).await;
        let (dispatch, mut rx) = dispatch_channel();
        close_iteration_canonical(&refresh, &dispatch, 1400)
            .await
            .expect("empty-member close succeeds");
        assert!(closed_iteration(&mut rx, 1400).complete);
    }

    #[tokio::test]
    async fn close_fires_storyclosed_without_member_promotion_mutations_1556() {
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, BoardState::Icebox, Some(("Pegasus", 100))),
        ])
        .await;
        let (dispatch, _rx) = dispatch_channel();
        let store = refresh.store();
        let mut rx = store.subscribe();
        close_iteration_canonical(&refresh, &dispatch, 1400)
            .await
            .unwrap();
        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }
        let member_promotion_touch = events.iter().any(|e| {
            matches!(
                e,
                crate::product_management::board::StoreChange::Event(
                    crate::product_management::board::StoryEvent::TagAdded { id, .. }
                        | crate::product_management::board::StoryEvent::StateTransitioned { id, .. }
                ) if *id == StoryId::new(200)
            )
        });
        assert!(
            !member_promotion_touch,
            "#1556: close promotes nothing — no member TagAdded/StateTransitioned: {events:?}"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                crate::product_management::board::StoreChange::Event(
                    crate::product_management::board::StoryEvent::StoryClosed { .. }
                )
            )),
            "StoryClosed still fires for the iteration-issue: {events:?}"
        );
    }
}
