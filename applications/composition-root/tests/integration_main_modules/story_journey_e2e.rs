#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, extract_story_id};
use std::time::Duration;

const POLL_ATTEMPTS: u32 = 600;
const POLL_INTERVAL: Duration = Duration::from_millis(100);

async fn workflow_state(bridge: &TestBridge, id: u64) -> String {
    let story = bridge
        .call_tool("get_story", serde_json::json!({"id": id}))
        .await
        .expect("get_story");
    story
        .get("workflow_state")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("get_story carries workflow_state; full={story}"))
        .to_string()
}

async fn wait_until_serving_reads(bridge: &TestBridge, who: &str) {
    for _ in 0..POLL_ATTEMPTS {
        if bridge
            .call_tool("view_backlog", serde_json::json!({}))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
    panic!("{who} bridge never started serving reads");
}

async fn wait_for_workflow_state(bridge: &TestBridge, id: u64, expected: &str) {
    for _ in 0..POLL_ATTEMPTS {
        if let Ok(story) = bridge
            .call_tool("get_story", serde_json::json!({"id": id}))
            .await
            && story.get("workflow_state").and_then(|v| v.as_str()) == Some(expected)
        {
            return;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
    panic!("#{id} never reached workflow_state={expected}");
}

#[test]
fn a_story_journeys_icebox_to_accepted_surviving_both_failure_loops() {
    let bpmux = TestBpmux::start_with_envs("human", &[("BPMUX_TEST_MODE", "1")]);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        wait_until_serving_reads(&pm, "pm").await;
        wait_until_serving_reads(&dev, "dev").await;
        wait_until_serving_reads(&designer, "designer").await;

        let id = create_target_story(
            &pm,
            "[Feature] journey-walk-target",
            "reject if one story cannot travel icebox to accepted through rework and rejection",
        )
        .await;
        assert_eq!(
            workflow_state(&pm, id).await,
            "icebox",
            "a new story is born in the icebox"
        );

        let created = pm
            .call_tool("create_iteration", serde_json::json!({"name": "Journey"}))
            .await
            .expect("pm creates the iteration");
        let iteration_id = extract_story_id(created.as_str().unwrap_or_default());
        assert_ne!(
            iteration_id, id,
            "the iteration allocates its own number; create said: {created}"
        );
        pm.call_tool(
            "designate_current_iteration",
            serde_json::json!({"iteration_id": iteration_id}),
        )
        .await
        .expect("pm designates the current iteration");
        pm.call_tool(
            "add_story_to_iteration",
            serde_json::json!({"iteration_id": iteration_id, "story_id": id, "position": 1}),
        )
        .await
        .expect("pm schedules the story into the current iteration");
        pm.call_tool(
            "move_story",
            serde_json::json!({
                "id": id,
                "target_state": "ready",
                "rationale": "scheduled into the current iteration for the journey walk"
            }),
        )
        .await
        .expect("pm promotes the scheduled story to ready");
        assert_eq!(
            workflow_state(&pm, id).await,
            "ready",
            "scheduling lands the story in ready"
        );

        let pickup = dev
            .call_tool(
                "start_story",
                serde_json::json!({
                    "id": id,
                    "pickup_paraphrase": "carry one story through the whole team workflow",
                    "falsifier_reframing": "reject if any leg of the journey refuses a canonical walk"
                }),
            )
            .await
            .expect("dev picks up from ready");
        let pickup_msg = pickup.as_str().unwrap_or_default().to_string();
        assert!(
            !pickup_msg.contains("REWORK"),
            "a first pickup carries no rework context; got {pickup_msg}"
        );

        dev.call_tool(
            "deliver_story",
            serde_json::json!({
                "id": id,
                "verified_observation": "walked the built slice end-to-end in the running app"
            }),
        )
        .await
        .expect("dev delivers into desk-check");
        assert_eq!(
            workflow_state(&pm, id).await,
            "desk-check",
            "deliver holds the story at desk-check"
        );

        designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({
                    "id": id,
                    "reason": "keyboard focus is lost after the dialog closes"
                }),
            )
            .await
            .expect("designer fails the desk-check (first failure loop)");
        assert_eq!(
            workflow_state(&pm, id).await,
            "in progress",
            "a failed desk-check sends the story back to rework"
        );

        dev.call_tool(
            "deliver_story",
            serde_json::json!({
                "id": id,
                "verified_observation": "focus restored; re-walked the dialog close path in the app"
            }),
        )
        .await
        .expect("dev re-delivers after the rework");
        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_id": id,
                    "verdict": "verified",
                    "substrate_grounding": "journey walk: designer verified the fix at desk-check"
                }),
            )
            .await
            .expect("designer grounds the desk-check in a verdict");
        designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"id": id, "acceptor": "team", "would_file_bug": false}),
            )
            .await
            .expect("designer passes the desk-check");
        assert_eq!(
            workflow_state(&pm, id).await,
            "acceptance",
            "a passed desk-check awaits acceptance"
        );

        pm.call_tool(
            "reject_story",
            serde_json::json!({
                "id": id,
                "reason": "the user-job is not satisfied on narrow viewports"
            }),
        )
        .await
        .expect("pm rejects at acceptance (second failure loop)");
        assert_eq!(
            workflow_state(&pm, id).await,
            "ready",
            "a rejected story returns to ready for re-scheduling"
        );

        let repickup = dev
            .call_tool(
                "start_story",
                serde_json::json!({
                    "id": id,
                    "pickup_paraphrase": "rework the rejected story",
                    "falsifier_reframing": "reject if the narrow-viewport failure survives the rework"
                }),
            )
            .await
            .expect("dev re-picks the rejected story");
        let repickup_msg = repickup.as_str().unwrap_or_default().to_string();
        assert!(
            repickup_msg.contains("REWORK") && repickup_msg.contains("narrow viewports"),
            "re-pickup surfaces the rejection as rework context; got {repickup_msg}"
        );

        dev.call_tool(
            "deliver_story",
            serde_json::json!({
                "id": id,
                "verified_observation": "narrow-viewport layout verified in the running app"
            }),
        )
        .await
        .expect("dev delivers the rework");
        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_id": id,
                    "verdict": "verified",
                    "substrate_grounding": "journey walk: designer verified the reworked slice in the app"
                }),
            )
            .await
            .expect("designer grounds the reworked desk-check in a verdict");
        designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"id": id, "acceptor": "team", "would_file_bug": false}),
            )
            .await
            .expect("designer passes the reworked desk-check");

        dev.call_tool(
            "submit_verdict",
            serde_json::json!({
                "story_id": id,
                "verdict": "verified",
                "substrate_grounding": "journey walk: dev verified the reworked slice in the app"
            }),
        )
        .await
        .expect("dev casts acceptance evidence");

        let accepted = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"id": id, "would_file_bug": false}),
            )
            .await
            .expect("pm accepts the twice-reworked story");
        let accept_msg = accepted.as_str().unwrap_or_default().to_string();
        assert!(
            accept_msg.contains("accepted") && accept_msg.contains("closed"),
            "acceptance names the accept + close transitions; got {accept_msg}"
        );

        wait_for_workflow_state(&pm, id, "accepted").await;
        let story = pm
            .call_tool("get_story", serde_json::json!({"id": id}))
            .await
            .expect("get_story after the walk");
        assert_eq!(
            story.get("workflow_state").and_then(|v| v.as_str()),
            Some("accepted"),
            "the journey concludes accepted; full={story}"
        );
        assert_eq!(
            story.get("is_closed").and_then(|v| v.as_bool()),
            Some(true),
            "the accepted story reads closed; full={story}"
        );

        pm.close().await;
        dev.close().await;
        designer.close().await;
    });
}

#[test]
fn retiring_from_the_real_cli_ends_the_journey_closed() {
    let bpmux = TestBpmux::start_with_envs("human", &[("BPMUX_TEST_MODE", "1")]);
    bpmux.simulate_secret();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        wait_until_serving_reads(&pm, "pm").await;

        let id = create_target_story(
            &pm,
            "[Feature] journey-retire-target",
            "reject if the CLI surface cannot retire a story the MCP surface created",
        )
        .await;

        let bin = env!("CARGO_BIN_EXE_bpmux");
        let out = tokio::process::Command::new(bin)
            .args([
                "move",
                &id.to_string(),
                "--target-state",
                "retired",
                "--rationale",
                "the journey may end retired, driven from the CLI surface",
            ])
            .env("BPMUX_PORT", bpmux.port().to_string())
            .env("BPMUX_ROLE", "pm")
            .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
            .output()
            .await
            .expect("run bpmux move subprocess");
        assert!(
            out.status.success(),
            "bpmux move --target-state retired succeeds through the same gated path as MCP; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        wait_for_workflow_state(&pm, id, "retired").await;
        let story = pm
            .call_tool("get_story", serde_json::json!({"id": id}))
            .await
            .expect("get_story after the CLI retire");
        assert_eq!(
            story.get("workflow_state").and_then(|v| v.as_str()),
            Some("retired"),
            "the CLI retire concludes the story; full={story}"
        );
        assert_eq!(
            story.get("is_closed").and_then(|v| v.as_bool()),
            Some(true),
            "the retired story reads closed; full={story}"
        );

        pm.close().await;
    });
}

#[test]
fn a_rejected_acceptance_reopens_the_closed_story_for_rework() {
    use harness::mock_gh::{issue_json, mock_story_full};
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(700, "Journey reopen round-trip", &["delivered"], None);
    let issue_state = mock_story_full(&server, 700, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm").await;
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev").await;
        let mut ingested = false;
        for _ in 0..POLL_ATTEMPTS {
            if pm
                .call_tool("get_story", serde_json::json!({"id": 700}))
                .await
                .is_ok()
            {
                ingested = true;
                break;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        assert!(
            ingested,
            "the mock-GH story reaches the canonical store via ingest before the walk begins"
        );

        harness::story_helpers::seed_team_verifiable_evidence_with_mock_gh(&bpmux, 700)
            .await;

        let mut labeled = false;
        for _ in 0..POLL_ATTEMPTS {
            if issue_state
                .lock()
                .unwrap()
                .labels
                .iter()
                .any(|l| l == "awaiting team")
            {
                labeled = true;
                break;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        assert!(
            labeled,
            "the team-verifiable acceptor-label reached the GitHub mock (read-your-writes) before accept, so a slow-runner re-ingest cannot clobber the seeded acceptor"
        );

        let mut acceptor_in_store = false;
        for _ in 0..POLL_ATTEMPTS {
            if let Ok(story) = pm
                .call_tool("get_story", serde_json::json!({"id": 700}))
                .await
                && story
                    .get("labels")
                    .and_then(|v| v.as_array())
                    .is_some_and(|labels| {
                        labels
                            .iter()
                            .any(|l| l.get("name").and_then(|n| n.as_str()) == Some("awaiting team"))
                    })
            {
                acceptor_in_store = true;
                break;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        assert!(
            acceptor_in_store,
            "the canonical store carries the seeded team-acceptor (re-ingest from the now-correct mock cannot clobber it) before accept fires"
        );

        pm.call_tool(
            "accept_story",
            serde_json::json!({"id": 700, "would_file_bug": false}),
        )
        .await
        .expect("accept_story should succeed against the seeded story");

        let mut closed = false;
        for _ in 0..POLL_ATTEMPTS {
            if issue_state.lock().unwrap().is_closed {
                closed = true;
                break;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        assert!(
            closed,
            "accepting pushes the close to GitHub — StoryClosed lands at the adapter, not just the store"
        );

        pm.call_tool(
            "reject_story",
            serde_json::json!({
                "id": 700,
                "reason": "accepted in error — the walk never covered the narrow viewport"
            }),
        )
        .await
        .expect("reject after accept should succeed (the reopen round-trip)");

        let mut reopened = false;
        for _ in 0..POLL_ATTEMPTS {
            if !issue_state.lock().unwrap().is_closed {
                reopened = true;
                break;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        assert!(
            reopened,
            "rejecting a concluded story pushes the reopen to GitHub — StoryReopened lands at the adapter"
        );

        let mut reject_visible = false;
        for _ in 0..POLL_ATTEMPTS {
            if let Ok(story) = pm
                .call_tool("get_story", serde_json::json!({"id": 700}))
                .await
                && story.get("latest_reject").is_some_and(|r| !r.is_null())
            {
                reject_visible = true;
                break;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        assert!(
            reject_visible,
            "the reject gate-record is readable through get_story before re-pickup, so start_story's rework context cannot race a verdict-clobbering re-ingest"
        );

        let repickup = dev
            .call_tool(
                "start_story",
                serde_json::json!({
                    "id": 700,
                    "pickup_paraphrase": "rework the wrongly-accepted story",
                    "falsifier_reframing": "reject if the reopened story refuses a canonical pickup"
                }),
            )
            .await
            .expect("the reopened story is workable — dev picks it up again");
        let repickup_msg = repickup.as_str().unwrap_or_default().to_string();
        assert!(
            repickup_msg.contains("marked as in progress"),
            "the reopened story re-enters the middle; got {repickup_msg}"
        );
        assert!(
            repickup_msg.contains("REWORK"),
            "re-pickup surfaces the rejection as rework context; got {repickup_msg}"
        );

        pm.close().await;
        dev.close().await;
    });
}
