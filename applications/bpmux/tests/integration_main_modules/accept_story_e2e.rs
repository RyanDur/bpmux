#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, seed_team_verifiable_evidence};
use std::time::Duration;

#[test]
fn accept_story_pm_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] accept-target",
            "reject if accept_story doesn't fire signal + state transition + close",
        )
        .await;

        seed_team_verifiable_evidence(&bpmux, target_id).await;

        let result = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"id": target_id, "would_file_bug": false}),
            )
            .await
            .expect("accept_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_id}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("accepted") && msg.contains("closed"),
            "happy-path message names accept + close transitions; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn accept_story_designer_refused_per_workflow_substrate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] designer-accept-target",
            "reject if designer firing accept_story isn't substrate-refused per #1385 workflow.toml actor_roles",
        )
        .await;

        let err = designer
            .call_tool(
                "accept_story",
                serde_json::json!({"id": target_id, "would_file_bug": false}),
            )
            .await
            .expect_err("designer should be substrate-refused per workflow.toml actor_roles");

        assert!(
            err.contains("workflow-role-not-allowed"),
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"designer\""),
            "rejection cites the firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"accept\""),
            "rejection cites the attempted transition; got {err}"
        );
        assert!(
            err.contains("\"allowed_roles\""),
            "rejection cites the allowed-roles list; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn accept_story_rejects_unknown_role() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let unknown = TestBridge::connect(&bpmux, "intern").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = unknown
            .call_tool(
                "accept_story",
                serde_json::json!({"id": 100, "would_file_bug": false}),
            )
            .await
            .expect_err("unknown role should be rejected");

        assert!(
            err.contains("not a known role") || err.contains("expected one of"),
            "rejection cites the role-vocabulary; got {err}"
        );

        unknown.close().await;
    });
}

#[test]
fn accept_story_view_board_reflects_accepted_via_mocked_seed() {
    use harness::mock_gh::{issue_json, mock_story_full};
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(600, "Pre-seeded delivered", &["delivered"], None);
    let issue_state = mock_story_full(&server, 600, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        harness::story_helpers::seed_team_verifiable_evidence_with_mock_gh(&bpmux, &base_url, 600)
            .await;

        let mut pushed = false;
        for _ in 0..50 {
            if issue_state
                .lock()
                .unwrap()
                .labels
                .iter()
                .any(|l| l == "awaiting team")
            {
                pushed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(
            pushed,
            "the team-verifiable acceptor-label reached the GitHub mock (read-your-writes) before accept, so a slow-runner re-ingest cannot clobber the seeded acceptor"
        );
        tokio::time::sleep(Duration::from_millis(1500)).await;

        pm.call_tool(
            "accept_story",
            serde_json::json!({"id": 600, "would_file_bug": false}),
        )
        .await
        .expect("accept_story should succeed against mocked story");

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| view.to_string());
        assert!(
            view_text.contains("#600") && view_text.contains("ACCEPTED"),
            "view_board reflects accepted state for mocked story #600 post-accept_story; got:\n{view_text}"
        );

        pm.close().await;
    });
}

#[test]
fn accept_role_survives_ingest_poll_after_acceptor_label_seed_1550() {
    use harness::mock_gh::{issue_json, mock_story_full};
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(601, "1550-ingest-clobber-guard", &["delivered"], None);
    let issue_state = mock_story_full(&server, 601, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        harness::story_helpers::seed_team_verifiable_evidence_with_mock_gh(&bpmux, &base_url, 601)
            .await;

        let mut pushed = false;
        for _ in 0..50 {
            if issue_state
                .lock()
                .unwrap()
                .labels
                .iter()
                .any(|l| l == "awaiting team")
            {
                pushed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(
            pushed,
            "outbox pushed the acceptor-label to the GitHub mock (read-your-writes) before re-ingest"
        );

        tokio::time::sleep(Duration::from_millis(1500)).await;

        pm.call_tool(
            "accept_story",
            serde_json::json!({"id": 601, "would_file_bug": false}),
        )
        .await
        .expect(
            "accept survives the re-ingest poll: the stateful GET /issues mock reflects the pushed acceptor-label, so re-ingest dispatching Seeded into the central store does not clobber it to OwnerWalk/[owner]",
        );

        pm.close().await;
    });
}

#[test]
fn accept_story_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("accept_story", serde_json::json!({}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("'id'"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
