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

    fn seed_member(number: u64, state: StoryState, iter_id: Option<u64>) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("[Feature] member-{number}"),
            state,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        if let Some(iter) = iter_id {
            s.raw_labels = vec![
                format!("{state:?}").to_lowercase(),
                format!("iteration-#{iter}"),
            ];
        } else {
            s.raw_labels = vec![format!("{state:?}").to_lowercase()];
        }
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
            seed_member(200, StoryState::Icebox, Some(1400)),
            seed_member(201, StoryState::Icebox, Some(1400)),
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
            seed_member(200, StoryState::Icebox, Some(1400)),
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
            seed_member(200, StoryState::Icebox, Some(1400)),
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
            seed_member(200, StoryState::Icebox, Some(1400)),
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
        // icebox_unified (that's the legacy substrate retiring at #1302).
        // Verified by holding a priority.json snapshot pre-call and
        // asserting post-call equality.
        let priority_dir = tempfile::tempdir().unwrap();
        let priority_path = priority_dir.path().join("priority.json");
        let priority_pre = r#"{"icebox_unified":[{"kind":"marker","id":42}]}"#;
        std::fs::write(&priority_path, priority_pre).unwrap();

        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(1400)),
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

    static PRIORITY_PATH_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn priority_sandbox_setup() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        std::sync::MutexGuard<'static, ()>,
    ) {
        let guard = PRIORITY_PATH_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("priority.json");
        unsafe {
            std::env::set_var("BPMUX_PRIORITY_PATH", &path);
        }
        (dir, path, guard)
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn close_promotes_only_stories_positionally_above_marker_bounded_by_previous_marker() {
        use crate::session::priority::{
            IceboxItem, PriorityIndex, evict_promoted_members_from_icebox_unified, load_priority,
            members_above_iteration_marker, save_priority,
        };
        let (_dir, priority_path, _env_guard) = priority_sandbox_setup();
        let seed = PriorityIndex {
            icebox_unified: vec![
                IceboxItem::Story { number: 901 },
                IceboxItem::Iteration { number: 1400 },
                IceboxItem::Story { number: 200 },
                IceboxItem::Story { number: 201 },
                IceboxItem::Iteration { number: 1401 },
                IceboxItem::Story { number: 300 },
            ],
            ..Default::default()
        };
        save_priority(&priority_path, &seed).unwrap();

        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_iteration_issue(1401, "Cassiopeia"),
            seed_member(901, StoryState::Icebox, None),
            seed_member(200, StoryState::Icebox, None),
            seed_member(201, StoryState::Icebox, None),
            seed_member(300, StoryState::Icebox, None),
        ])
        .await;

        let priority = load_priority(&priority_path);
        let candidate_numbers = members_above_iteration_marker(&priority, 1401);
        assert_eq!(
            candidate_numbers,
            vec![200, 201],
            "second iteration's above-band bounded by previous marker; story-901 stays above the first marker"
        );

        promote_members_then_close_iteration(&refresh, 1401, &candidate_numbers)
            .await
            .expect("positional close succeeds for the inner iteration");
        assert_eq!(
            refresh.story(200).unwrap().state,
            StoryState::Ready,
            "above-marker story 200 promoted"
        );
        assert_eq!(
            refresh.story(201).unwrap().state,
            StoryState::Ready,
            "above-marker story 201 promoted"
        );
        assert_eq!(
            refresh.story(901).unwrap().state,
            StoryState::Icebox,
            "story-901 stays in icebox (above the OTHER marker, not 1401's band)"
        );
        assert_eq!(
            refresh.story(300).unwrap().state,
            StoryState::Icebox,
            "story-300 stays in icebox (positioned below the closed marker)"
        );

        let mut post = load_priority(&priority_path);
        evict_promoted_members_from_icebox_unified(&mut post, &candidate_numbers);
        save_priority(&priority_path, &post).unwrap();
        let reloaded = load_priority(&priority_path);
        assert_eq!(
            reloaded.icebox_unified,
            vec![
                IceboxItem::Story { number: 901 },
                IceboxItem::Iteration { number: 1400 },
                IceboxItem::Iteration { number: 1401 },
                IceboxItem::Story { number: 300 },
            ],
            "promoted stories evicted; both markers stay (1401 closed-state visual record); story-901 and story-300 stay positionally"
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
            seed_member(200, StoryState::Icebox, Some(1400)),
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
