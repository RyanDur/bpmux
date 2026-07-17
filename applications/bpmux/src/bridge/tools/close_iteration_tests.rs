#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(unsafe_code)]

mod canonical_via_gh_1300 {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::store::{AppAction, Dispatch};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use crate::transport::rest::iteration_close::{CloseIterationError, close_iteration};
    use crate::transport::rest::{close_iteration_canonical, collect_ready_queue_blockers};
    use chrono::{TimeZone, Utc};
    use product_management::iteration::{Iteration, IterationAction, IterationId};
    use product_management::story::{Story, StoryId, Work};
    use source_codec::mapper::decode_iteration_from_story;
    use std::sync::Arc;
    use tokio::sync::mpsc::UnboundedReceiver;

    fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
        Dispatch::recording()
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

    fn seed_member(id: u64, state: Work, iteration: Option<(&str, u32)>) -> (Story, Vec<String>) {
        let s = Story::new_minimal(
            StoryId::new(id),
            format!("[Feature] member-{id}"),
            state.clone(),
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        let state_label = format!("{state:?}").to_lowercase();
        let labels = match iteration {
            Some((name, position)) => {
                vec![state_label, format!("iteration-{name}.{position}")]
            }
            None => vec![state_label],
        };
        (s, labels)
    }

    fn seed_iteration_issue(id: u64, name: &str) -> (Story, Vec<String>) {
        let s = Story::new_minimal(
            StoryId::new(id),
            format!("Iteration {name}"),
            Work::NotStarted,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        (s, vec!["iteration".to_string()])
    }

    fn seed_current_iteration_issue_with_members(
        id: u64,
        name: &str,
        members: &[u64],
    ) -> (Story, Vec<String>) {
        use source_codec::relationships::{IterationRelationships, Relationships};
        let mut s = Story::new_minimal(
            StoryId::new(id),
            format!("Iteration {name}"),
            Work::NotStarted,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        s.body = Some(
            IterationRelationships {
                outcome: None,
                stories: members.iter().map(|n| StoryId::new(*n)).collect(),
            }
            .write_into_body(""),
        );
        (
            s,
            vec!["iteration".to_string(), "current-iteration".to_string()],
        )
    }

    async fn build_refresh(
        seeds: Vec<(Story, Vec<String>)>,
    ) -> (
        Arc<dyn RefreshDriver>,
        Arc<InMemorySourceAdapter>,
        crate::store::Select,
    ) {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        for (story, labels) in &seeds {
            adapter.seed_labels(story.id.value(), labels.clone());
        }
        adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let iterations: Vec<Iteration> = seeds
            .iter()
            .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
            .map(|(story, labels)| decode_iteration_from_story(story, labels, false))
            .collect();
        let select = crate::store::bind_seeded_refresh(&orch, vec![], iterations, vec![]);
        orch.refresh_all().await.unwrap();
        (orch, adapter, select)
    }

    #[tokio::test]
    async fn close_promotes_nothing_and_does_not_error_on_canonical_member_1556() {
        let canonical_member = Story::new_minimal(
            StoryId::new(200),
            "[Feature] canonical-member".to_string(),
            Work::NotStarted,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        let canonical_labels = vec![
            "icebox".to_string(),
            "parent-iteration-#1400.100".to_string(),
        ];
        let (refresh, _adapter, select) = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            (canonical_member, canonical_labels),
        ])
        .await;
        let (dispatch, mut rx) = dispatch_channel();

        close_iteration_canonical(&refresh, &dispatch, &select, 1400)
            .await
            .expect("close succeeds without erroring on a canonically-attributed member");

        assert!(
            closed_iteration(&mut rx, 1400).complete,
            "iteration closed — close is terminal (dispatched onto the slice as complete)"
        );
        let member = refresh.story(200).expect("member 200 exists");
        assert_eq!(
            member.state,
            Work::NotStarted,
            "#1556: close promotes NOTHING — the member stays in its state, not moved to ready"
        );
    }

    #[tokio::test]
    async fn closing_the_current_iteration_clears_its_designation_1540() {
        let (current, mut current_labels) = seed_iteration_issue(1400, "Pegasus");
        current_labels.push("current-iteration".to_string());
        let (refresh, _adapter, select) = build_refresh(vec![(current, current_labels)]).await;
        assert!(
            select
                .select(|state| {
                    product_management::iteration::get(&state.iterations, IterationId::new(1400))
                })
                .expect("iteration exists")
                .is_current,
            "precondition: iteration #1400 is the current designation"
        );
        let (dispatch, mut rx) = dispatch_channel();

        close_iteration_canonical(&refresh, &dispatch, &select, 1400)
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
    async fn ready_queue_empty_guard_blocks_close_when_a_current_iteration_member_is_ready() {
        let (_refresh, _adapter, select) = build_refresh(vec![
            seed_current_iteration_issue_with_members(1400, "Pegasus", &[900]),
            seed_member(900, Work::NotStarted, None),
        ])
        .await;
        let blockers = collect_ready_queue_blockers(&select);
        assert_eq!(
            blockers.len(),
            1,
            "a not-started current-iteration member projects Ready and surfaces as a blocker"
        );
        assert_eq!(blockers[0].id, 900);
        assert_eq!(blockers[0].state, "ready");
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_excludes_a_not_started_non_member() {
        let (_refresh, _adapter, select) = build_refresh(vec![
            seed_current_iteration_issue_with_members(1400, "Pegasus", &[]),
            seed_member(900, Work::NotStarted, None),
        ])
        .await;
        let blockers = collect_ready_queue_blockers(&select);
        assert!(
            blockers.is_empty(),
            "a not-started story outside the current iteration derives Icebox and never blocks close: {blockers:?}"
        );
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_passes_when_no_ready_stories() {
        let (_refresh, _adapter, select) = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, Work::NotStarted, Some(("Pegasus", 100))),
        ])
        .await;
        let blockers = collect_ready_queue_blockers(&select);
        assert!(
            blockers.is_empty(),
            "no ready blockers — close proceeds: {blockers:?}"
        );
    }

    #[tokio::test]
    async fn closed_iteration_issue_remains_queryable_for_history_preservation() {
        let (refresh, _adapter, select) = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, Work::NotStarted, Some(("Pegasus", 100))),
        ])
        .await;
        let (dispatch, mut rx) = dispatch_channel();
        close_iteration_canonical(&refresh, &dispatch, &select, 1400)
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
        let (refresh, _adapter, select) =
            build_refresh(vec![seed_iteration_issue(1400, "Pegasus")]).await;
        let (dispatch, mut rx) = dispatch_channel();
        close_iteration_canonical(&refresh, &dispatch, &select, 1400)
            .await
            .expect("empty-member close succeeds");
        assert!(closed_iteration(&mut rx, 1400).complete);
    }

    #[tokio::test]
    async fn close_fires_storyclosed_without_member_promotion_mutations_1556() {
        let (refresh, adapter, select) = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, Work::NotStarted, Some(("Pegasus", 100))),
        ])
        .await;
        let (dispatch, mut rx) = dispatch_channel();
        let before = refresh.story(200).expect("member 200 seeded");
        let labels_before = adapter.labels_for(200);
        close_iteration_canonical(&refresh, &dispatch, &select, 1400)
            .await
            .unwrap();
        let after = refresh.story(200).expect("member 200 survives the close");
        assert_eq!(
            after.state, before.state,
            "#1556: close promotes nothing — no member StateTransitioned"
        );
        assert_eq!(
            adapter.labels_for(200),
            labels_before,
            "#1556: close promotes nothing — no member TagAdded/TagRemoved"
        );
        assert!(
            closed_iteration(&mut rx, 1400).complete,
            "the close still lands for the iteration-issue — dispatched onto the slice as complete"
        );
    }

    fn test_hub() -> (Arc<crate::hub::Hub>, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let (tx, _rx) = tokio::sync::broadcast::channel(16);
        let hub = Arc::new(crate::hub::Hub::new(
            dir.path().to_path_buf(),
            "test".to_string(),
            tx,
        ));
        (hub, dir)
    }

    #[tokio::test]
    async fn closing_an_id_that_is_a_story_rather_than_an_iteration_names_it_as_such() {
        let (refresh, _adapter, select) = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, Work::NotStarted, None),
        ])
        .await;
        let (dispatch, _rx) = dispatch_channel();
        let (hub, _dir) = test_hub();

        let error = close_iteration(&hub, &refresh, &dispatch, &select, 200)
            .await
            .expect_err("#200 is a story, not an iteration");

        assert!(
            matches!(error, CloseIterationError::NotAnIteration(200)),
            "an id the store holds as a story is refused as NOT-AN-ITERATION, which is a different \
             answer from never having heard of it; got {error:?}"
        );
    }

    #[tokio::test]
    async fn closing_an_id_the_store_has_never_heard_of_says_not_found() {
        let (refresh, _adapter, select) =
            build_refresh(vec![seed_iteration_issue(1400, "Pegasus")]).await;
        let (dispatch, _rx) = dispatch_channel();
        let (hub, _dir) = test_hub();

        let error = close_iteration(&hub, &refresh, &dispatch, &select, 9999)
            .await
            .expect_err("#9999 is absent from the store");

        assert!(
            matches!(error, CloseIterationError::NotFound(9999)),
            "an id absent from the store is NOT-FOUND, not mislabelled as a non-iteration; got {error:?}"
        );
    }
}
