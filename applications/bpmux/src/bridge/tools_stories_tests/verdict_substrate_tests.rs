#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use protocol::ClientMessage;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod verdict_substrate_integration_1407 {
    use crate::bridge::tools::handle_submit_verdict;
    use crate::bridge::tools_stories::handle_accept_story;
    use crate::bridge::tools_stories::role_from_name;
    use crate::hub::Hub;
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::store::{AppAction, SeededStore};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::evidence::{EvidenceAction, evidence_for_story};
    use product_management::story::{Story, StoryId, Work};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::broadcast;

    fn make_hub(dir: PathBuf) -> Arc<Hub> {
        let (tx, _) = broadcast::channel(16);
        Arc::new(Hub::new(dir, "owner".to_string(), tx))
    }

    fn fresh_store() -> SeededStore {
        SeededStore::new(vec![], vec![], vec![])
    }

    fn orch_and_store() -> (Arc<dyn RefreshDriver>, SeededStore) {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let store = SeededStore::new(vec![], vec![], vec![]);
        store.bind_refresh(&orch);
        (orch, store)
    }

    async fn orch_with_story_and_store(seeded: Story) -> (Arc<dyn RefreshDriver>, SeededStore) {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![seeded]);
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let store = SeededStore::new(vec![], vec![], vec![]);
        store.bind_refresh(&orch);
        orch.refresh_all().await.unwrap();
        (orch, store)
    }

    async fn submit(
        store: &SeededStore,
        hub: &Arc<Hub>,
        args: &serde_json::Map<String, serde_json::Value>,
        role: &str,
    ) -> Result<String, String> {
        handle_submit_verdict(
            args,
            role,
            role_from_name(role),
            hub,
            &store.dispatcher(),
            &store.select(),
        )
        .await
    }

    fn delivered_story(id: u64) -> Story {
        let mut story = Story::new_minimal(
            StoryId::new(id),
            format!("story-{id}"),
            Work::Acceptance,
            Utc.with_ymd_and_hms(2026, 5, 18, 0, 0, 0).unwrap(),
            "dev".to_string(),
        );
        story.acceptor = Some(product_management::story::Acceptor::Team);
        story
    }

    fn seed_expired_verdict(store: &SeededStore, id: u64, role: &str) {
        store.dispatch(AppAction::Evidence(EvidenceAction::VerdictSubmitted {
            story_id: StoryId::new(id),
            from_role: role.to_string(),
            verdict: "verified".to_string(),
            substrate_grounding: "src/foo.rs cite".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 5, 18, 0, 0, 0).unwrap(),
            grounded_head: None,
        }));
    }

    fn seed_verdict_grounded_on(store: &SeededStore, id: u64, role: &str, head: &str) {
        store.dispatch(AppAction::Evidence(EvidenceAction::VerdictSubmitted {
            story_id: StoryId::new(id),
            from_role: role.to_string(),
            verdict: "verified".to_string(),
            substrate_grounding: "src/foo.rs cite".to_string(),
            ts: Utc::now(),
            grounded_head: Some(head.to_string()),
        }));
    }

    fn verdict_args(id: u64) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert("story_id".into(), serde_json::json!(id));
        m.insert("verdict".into(), serde_json::json!("verified"));
        m.insert(
            "substrate_grounding".into(),
            serde_json::json!("src/foo.rs cite"),
        );
        m
    }

    fn accept_args(id: u64) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert("id".into(), serde_json::json!(id));
        m.insert("would_file_bug".into(), serde_json::json!(false));
        m
    }

    #[tokio::test]
    async fn submit_verdict_records_both_roles_in_the_store() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let store = fresh_store();
        submit(&store, &hub, &verdict_args(42), "designer:claude-code")
            .await
            .expect("designer submit_verdict");
        submit(&store, &hub, &verdict_args(42), "dev:claude-code")
            .await
            .expect("dev submit_verdict");
        let recorded = store
            .select()
            .select(|state| evidence_for_story(&state.evidence, StoryId::new(42)));
        assert_eq!(recorded.len(), 2);
        let roles: Vec<&str> = recorded.iter().map(|v| v.from_role.as_str()).collect();
        assert!(roles.contains(&"designer"));
        assert!(roles.contains(&"dev"));
    }

    #[tokio::test]
    async fn accept_story_succeeds_when_both_verdicts_present_for_target_story() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (orch, store) = orch_with_story_and_store(delivered_story(42)).await;
        submit(&store, &hub, &verdict_args(42), "designer")
            .await
            .unwrap();
        submit(&store, &hub, &verdict_args(42), "dev")
            .await
            .unwrap();

        let result =
            handle_accept_story(&accept_args(42), "pm", "pm", None, Some(&orch), Some(&hub)).await;
        assert!(
            result.is_ok(),
            "accept_story must succeed when both verdicts present for target story; got {result:?}"
        );
    }

    #[tokio::test]
    async fn accept_reprompts_each_role_whose_verdict_has_expired() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (orch, store) = orch_with_story_and_store(delivered_story(42)).await;
        seed_expired_verdict(&store, 42, "designer");
        seed_expired_verdict(&store, 42, "dev");

        let mut rx = hub.subscribe();
        let err = handle_accept_story(&accept_args(42), "pm", "pm", None, Some(&orch), Some(&hub))
            .await
            .expect_err("expired verdicts must still block the accept");
        assert!(
            err.contains("verdict-expired"),
            "block still cites expiry — the reprompt is additive, never a bypass; got {err}"
        );

        let mut reprompts = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if msg.text.contains("submit_verdict") {
                reprompts.push(msg);
            }
        }
        assert_eq!(
            reprompts.len(),
            2,
            "each expired role is reprompted — and it REACHES the user via hub.broadcast on the live (empty_ws=None) path, not a dropped ws-signal; got {reprompts:?}"
        );
        let addressees: Vec<String> = reprompts
            .iter()
            .filter_map(|m| m.addressee.clone())
            .collect();
        assert!(
            addressees.iter().any(|a| a.contains("designer"))
                && addressees.iter().any(|a| a.contains("dev")),
            "both expired roles are addressed by name; got {addressees:?}"
        );
        assert!(
            reprompts.iter().all(|m| m.from == "bpmux"),
            "the reprompt is a bpmux system nudge (from=bpmux); got {reprompts:?}"
        );
    }

    #[tokio::test]
    async fn view_verdicts_marks_verdict_expired_when_grounded_head_moved() {
        let dir = TempDir::new().unwrap();
        let _hub = make_hub(dir.path().to_path_buf());
        let (_orch, store) = orch_with_story_and_store(delivered_story(42)).await;
        seed_verdict_grounded_on(
            &store,
            42,
            "designer",
            "a-frozen-sha-that-is-not-current-head",
        );

        let response = crate::bridge::tools::handle_view_verdicts(&view_args(42), &store.select())
            .await
            .expect("view_verdicts succeeds");
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("response is JSON");
        let records = parsed.as_array().expect("response is array");
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0]["substrate_changed"],
            serde_json::json!(true),
            "a verdict grounded on a head that is no longer current reads substrate_changed; got {records:?}"
        );
        assert_eq!(
            records[0]["fresh"],
            serde_json::json!(false),
            "head-moved expiry is visible at the read-axis, not only enforced at the gate; got {records:?}"
        );
    }

    #[tokio::test]
    async fn accept_blocks_and_reprompts_when_a_grounded_head_has_moved() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (orch, store) = orch_with_story_and_store(delivered_story(42)).await;
        seed_verdict_grounded_on(&store, 42, "designer", "stale-sha-x");
        seed_verdict_grounded_on(&store, 42, "dev", "stale-sha-x");

        let mut rx = hub.subscribe();
        let err = handle_accept_story(&accept_args(42), "pm", "pm", None, Some(&orch), Some(&hub))
            .await
            .expect_err(
                "a moved grounded-head must block accept even though the verdicts are time-fresh",
            );
        assert!(
            err.contains("verdict-expired"),
            "substrate-change expiry routes through the same expired path; got {err}"
        );

        let mut reprompts = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if msg.text.contains("submit_verdict") {
                reprompts.push(msg);
            }
        }
        assert_eq!(
            reprompts.len(),
            2,
            "both head-stale roles are reprompted against the new head — reaching the user via hub.broadcast on the live (empty_ws=None) path; got {reprompts:?}"
        );
    }

    #[tokio::test]
    async fn accept_story_refuses_when_verdicts_exist_only_on_a_different_story() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (orch, store) = orch_with_story_and_store(delivered_story(42)).await;
        submit(&store, &hub, &verdict_args(99), "designer")
            .await
            .unwrap();
        submit(&store, &hub, &verdict_args(99), "dev")
            .await
            .unwrap();

        let err = handle_accept_story(&accept_args(42), "pm", "pm", None, Some(&orch), Some(&hub))
            .await
            .expect_err("verdicts on #99 must not satisfy accept_story #42");
        assert!(
            err.contains("workflow-gate-mismatch"),
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("designer-verdict") && err.contains("dev-verdict"),
            "both verdicts missing for the target story; got {err}"
        );
    }

    #[tokio::test]
    async fn accept_story_refuses_when_only_designer_verdict_present() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (orch, store) = orch_with_story_and_store(delivered_story(42)).await;
        submit(&store, &hub, &verdict_args(42), "designer")
            .await
            .unwrap();

        let err = handle_accept_story(&accept_args(42), "pm", "pm", None, Some(&orch), Some(&hub))
            .await
            .expect_err("missing dev-verdict must refuse");
        assert!(
            err.contains("dev-verdict"),
            "rejection cites missing dev-verdict; got {err}"
        );
        assert!(
            !err.contains("designer-verdict"),
            "rejection should NOT cite designer-verdict (it's present); got {err}"
        );
    }

    #[tokio::test]
    async fn submit_verdict_broadcasts_pm_addressed_system_on_success() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let store = fresh_store();
        let mut rx = hub.subscribe();
        submit(&store, &hub, &verdict_args(42), "designer:claude-code")
            .await
            .expect("designer submit_verdict");
        let msg = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .expect("notification arrives within ws-latency")
            .expect("broadcast channel open");
        assert_eq!(msg.from, "bpmux");
        assert_eq!(msg.addressee.as_deref(), Some("pm"));
        assert_eq!(
            msg.message_type,
            message::MessageType::System,
            "#1494 producer-axis canon: bpmux verdict-cast must wire as MessageType::System so renderer classOf-remap classifies as bpmux-class; the prior Chat default leaked past the bpmux pill (sister to #1456 filter-canon)"
        );
        assert!(
            msg.text.contains("#42"),
            "payload carries story id; got {}",
            msg.text
        );
        assert!(
            msg.text.contains("designer"),
            "payload carries from_role; got {}",
            msg.text
        );
        assert!(
            msg.text.contains("verified"),
            "payload carries verdict; got {}",
            msg.text
        );
    }

    #[tokio::test]
    async fn submit_verdict_broadcasts_exactly_once_per_fire() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let store = fresh_store();
        let mut rx = hub.subscribe();
        submit(&store, &hub, &verdict_args(42), "designer:claude-code")
            .await
            .expect("designer submit_verdict");
        let _first = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .expect("first notification arrives")
            .expect("broadcast channel open");
        let second = tokio::time::timeout(std::time::Duration::from_millis(150), rx.recv()).await;
        assert!(
            second.is_err(),
            "no duplicate notification fires for single submit_verdict; got {second:?}"
        );
    }

    #[tokio::test]
    async fn submit_verdict_broadcasts_replacement_on_replay() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let store = fresh_store();
        let mut rx = hub.subscribe();
        submit(&store, &hub, &verdict_args(42), "designer:claude-code")
            .await
            .expect("first submit_verdict");
        let first = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .expect("first notification arrives")
            .expect("broadcast channel open");
        assert!(first.text.contains("verified"));

        let mut second_args = serde_json::Map::new();
        second_args.insert("story_id".into(), serde_json::json!(42));
        second_args.insert("verdict".into(), serde_json::json!("rejected"));
        second_args.insert(
            "substrate_grounding".into(),
            serde_json::json!("updated cite after second pass"),
        );
        submit(&store, &hub, &second_args, "designer:claude-code")
            .await
            .expect("replay submit_verdict");
        let replacement = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .expect("replacement notification arrives")
            .expect("broadcast channel open");
        assert!(
            replacement.text.contains("rejected"),
            "replacement notification reflects new verdict; got {}",
            replacement.text
        );
    }

    #[tokio::test]
    async fn submit_verdict_does_not_broadcast_when_substrate_grounding_missing() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let store = fresh_store();
        let mut rx = hub.subscribe();
        let mut bad_args = serde_json::Map::new();
        bad_args.insert("story_id".into(), serde_json::json!(42));
        bad_args.insert("verdict".into(), serde_json::json!("verified"));
        let err = submit(&store, &hub, &bad_args, "designer:claude-code")
            .await
            .expect_err("missing substrate_grounding must refuse");
        assert!(err.contains("substrate_grounding"));
        let nothing = tokio::time::timeout(std::time::Duration::from_millis(150), rx.recv()).await;
        assert!(
            nothing.is_err(),
            "no notification fires on submit_verdict failure; got {nothing:?}"
        );
    }

    #[tokio::test]
    async fn submit_verdict_does_not_broadcast_when_role_refused() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let store = fresh_store();
        let mut rx = hub.subscribe();
        let err = submit(&store, &hub, &verdict_args(42), "pm:claude-code")
            .await
            .expect_err("pm role must refuse");
        assert!(err.contains("restricted to"));
        let nothing = tokio::time::timeout(std::time::Duration::from_millis(150), rx.recv()).await;
        assert!(
            nothing.is_err(),
            "no notification fires on role-refusal; got {nothing:?}"
        );
    }

    fn view_args(id: u64) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert("story_id".into(), serde_json::json!(id));
        m
    }

    #[tokio::test]
    async fn view_verdicts_returns_record_after_submit() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (_orch, store) = orch_and_store();
        submit(&store, &hub, &verdict_args(42), "designer:claude-code")
            .await
            .expect("designer submit_verdict");
        let response = crate::bridge::tools::handle_view_verdicts(&view_args(42), &store.select())
            .await
            .expect("view_verdicts succeeds after write");
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("response is JSON");
        let records = parsed.as_array().expect("response is array");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["from_role"], "designer");
        assert_eq!(records[0]["verdict"], "verified");
        assert_eq!(records[0]["substrate_grounding"], "src/foo.rs cite");
        assert!(records[0]["ts"].is_string());
        assert_eq!(records[0]["story_id"], 42);
    }

    #[tokio::test]
    async fn view_verdicts_returns_empty_for_unknown_story() {
        let (_orch, store) = orch_and_store();
        let response =
            crate::bridge::tools::handle_view_verdicts(&view_args(99999), &store.select())
                .await
                .expect("view_verdicts succeeds for empty store");
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("response is JSON");
        let records = parsed.as_array().expect("response is array");
        assert!(
            records.is_empty(),
            "empty store should return empty array; got {records:?}"
        );
    }

    #[tokio::test]
    async fn view_verdicts_scopes_by_story_id() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (_orch, store) = orch_and_store();
        submit(&store, &hub, &verdict_args(10), "designer:claude-code")
            .await
            .unwrap();
        submit(&store, &hub, &verdict_args(20), "dev:claude-code")
            .await
            .unwrap();
        submit(&store, &hub, &verdict_args(30), "designer:claude-code")
            .await
            .unwrap();

        let response = crate::bridge::tools::handle_view_verdicts(&view_args(20), &store.select())
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        let records = parsed.as_array().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["story_id"], 20);
        assert_eq!(records[0]["from_role"], "dev");
    }

    #[tokio::test]
    async fn view_verdicts_returns_both_roles_for_one_story() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (_orch, store) = orch_and_store();
        submit(&store, &hub, &verdict_args(42), "designer:claude-code")
            .await
            .unwrap();
        submit(&store, &hub, &verdict_args(42), "dev:claude-code")
            .await
            .unwrap();
        let response = crate::bridge::tools::handle_view_verdicts(&view_args(42), &store.select())
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        let records = parsed.as_array().unwrap();
        assert_eq!(records.len(), 2);
        let roles: Vec<&str> = records
            .iter()
            .filter_map(|r| r["from_role"].as_str())
            .collect();
        assert!(roles.contains(&"designer"));
        assert!(roles.contains(&"dev"));
    }

    #[tokio::test]
    async fn view_verdicts_is_pure_read() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (_orch, store) = orch_and_store();
        submit(&store, &hub, &verdict_args(42), "designer:claude-code")
            .await
            .unwrap();
        let count_before = store
            .select()
            .select(|state| evidence_for_story(&state.evidence, StoryId::new(42)))
            .len();

        crate::bridge::tools::handle_view_verdicts(&view_args(42), &store.select())
            .await
            .unwrap();
        crate::bridge::tools::handle_view_verdicts(&view_args(42), &store.select())
            .await
            .unwrap();
        crate::bridge::tools::handle_view_verdicts(&view_args(42), &store.select())
            .await
            .unwrap();

        let count_after = store
            .select()
            .select(|state| evidence_for_story(&state.evidence, StoryId::new(42)))
            .len();
        assert_eq!(
            count_before, count_after,
            "view_verdicts must not mutate the store"
        );
    }

    #[tokio::test]
    async fn view_verdicts_refuses_missing_story_id() {
        let (_orch, store) = orch_and_store();
        let empty_args = serde_json::Map::new();
        let err = crate::bridge::tools::handle_view_verdicts(&empty_args, &store.select())
            .await
            .expect_err("missing story_id must refuse");
        assert!(err.contains("story_id"));
    }

    #[tokio::test]
    async fn view_verdicts_surfaces_freshness_state_at_the_read_axis() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let (_orch, store) = orch_with_story_and_store(delivered_story(42)).await;
        submit(&store, &hub, &verdict_args(42), "designer:claude-code")
            .await
            .expect("designer submit_verdict");
        let response = crate::bridge::tools::handle_view_verdicts(&view_args(42), &store.select())
            .await
            .expect("view_verdicts succeeds with the board store");
        let parsed: serde_json::Value = serde_json::from_str(&response).expect("response is JSON");
        let records = parsed.as_array().expect("response is array");
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0]["fresh"],
            serde_json::json!(true),
            "a just-cast verdict is within the accept-window → fresh shown at the read-axis (not just presence); got {records:?}"
        );
        assert!(
            records[0]["expires_at"].is_string(),
            "freshness must surface expires_at (cast_at + accept-window) in view_verdicts; got {records:?}"
        );
    }
}
