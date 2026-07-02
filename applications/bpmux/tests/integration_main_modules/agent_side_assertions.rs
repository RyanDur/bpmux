//! Agent-side assertion tests for story #199.
//!
//! These verify what an MCP agent RECEIVES through the bridge's tool
//! responses — closing the two-audience testing gap. The existing
//! harness supports human-side assertions (screen capture via
//! `TestBpmux`); these tests prove the symmetric agent-side path
//! works via `TestBridge::call_tool`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

/// Presence-wait timeout for integration tests (#276). See
/// bridge_acceptance_test.rs for the rationale — GH Actions +
/// subprocess spawn needs more headroom than 5s.
const WAIT_PRESENCE_TIMEOUT: Duration = Duration::from_secs(30);

/// AC: "Given a test calls view_board as a specific agent role,
/// when stories exist in the board, then the response contains
/// stories with number, title, and state — matching what that agent
/// would see via MCP."
///
/// Verifies that `call_tool` parses the JSON response and exposes
/// typed fields for assertion. Seeds deterministic stories via
/// httpmock-backed GitHubSourceAdapter so the test runs without a
/// live gh dependency yet exercises the real-substrate cascade per
/// "only octocrab as test-double" directive.
#[test]
fn view_board_response_exposes_story_fields_for_agent_assertions() {
    use harness::mock_gh::issue_json;
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                Value::Array(vec![
                    issue_json(100, "Ready one", &["ready"], None),
                    issue_json(101, "Delivered one", &["delivered"], None),
                ])
                .to_string(),
            );
    });

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let board = dev
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");

        let rendered = board.as_str().unwrap_or("");
        assert!(
            rendered.contains("#100") && rendered.contains("Ready one"),
            "view_board should expose story 100 fields. got:\n{rendered}"
        );
        assert!(
            rendered.contains("#101") && rendered.contains("Delivered one"),
            "view_board should expose story 101 fields. got:\n{rendered}"
        );
        assert!(
            rendered.contains("READY (") && rendered.contains("ACCEPTANCE ("),
            "view_board should group stories by state. got:\n{rendered}"
        );
        dev.close().await;
    });
}

/// AC: "Given a test triggers deliver_story, when the signal fires,
/// then the designer-role response contains the desk-check signal
/// and the dev-role response does NOT contain the desk-check signal."
///
/// deliver_story fires the `desk-check needed` workflow signal to
/// designer; dev receives the separate `queue update` signal. This
/// test verifies role-targeted routing from the agent side — the
/// harness gap that motivated #199.
#[test]
fn deliver_story_signal_routes_desk_check_to_designer_not_dev() {
    use harness::mock_gh::{issue_json, mock_story_mutations};
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let body = format!(
        "## Acceptance test path\n\n{}/tests/harness/story_helpers.rs\n",
        env!("CARGO_MANIFEST_DIR")
    );
    let issue = issue_json(199, "deliver-target", &["in progress"], Some(&body));
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue.clone()]).to_string());
    });
    mock_story_mutations(&server, 199, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        let designer = TestBridge::connect_with_mock_gh(&bpmux, "designer", &base_url).await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let designer_sub = designer.subscribe("addressed_to_me").await;
        let dev_sub = dev.subscribe("addressed_to_me").await;

        // deliver_story's API returns Err only on signal-send failure
        // (load-bearing). gh label edits are reported in the Ok
        // status string, so the test asserts that signals succeeded
        // (Ok) without masking the gh outcome. This removes the
        // earlier `let _ =` variance tolerance — the test now takes
        // one execution path regardless of whether gh is available.
        let status = dev
            .call_tool(
                "deliver_story",
                serde_json::json!({
                    "id": 199,
                    "title": "test api agent assertions",
                    "verified_observation": "drove agent_side_assertions test end-to-end; designer received desk-check signal, dev did not"
                }),
            )
            .await
            .expect("deliver_story signals should succeed");
        let status_str = status.as_str().unwrap_or("");
        assert!(
            status_str.contains("signals sent"),
            "deliver_story Ok status should report signals sent. got: {status_str}"
        );

        let designer_event = designer.wait_for_event(&designer_sub, WAIT_PRESENCE_TIMEOUT).await;
        assert!(
            designer_event.contains("#199 (deliver-target) delivered"),
            "designer must receive the delivery signal on the live addressed_to_me push; got {designer_event}"
        );

        let dev_event = dev.wait_for_event(&dev_sub, Duration::from_millis(1500)).await;
        assert!(
            !dev_event.contains("#199 (deliver-target) delivered"),
            "dev (the sender, not addressed) must NOT receive the designer-addressed delivery signal; got {dev_event}"
        );

        dev.close().await;
        designer.close().await;
    });
}

#[test]
fn pass_desk_check_signal_routes_to_owner_not_dev() {
    use harness::mock_gh::{issue_json, mock_story_mutations};
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(203, "pass-desk-check-target", &["desk-check"], None);
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue.clone()]).to_string());
    });
    mock_story_mutations(&server, 203, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect_with_mock_gh(&bpmux, "designer", &base_url).await;
        let owner = TestBridge::connect_with_mock_gh(&bpmux, "owner", &base_url).await;
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_id": 203,
                    "verdict": "verified",
                    "substrate_grounding": "rg + git show fragments cited at src/foo.rs:42"
                }),
            )
            .await
            .expect("designer submit_verdict");

        let owner_sub = owner.subscribe("addressed_to_me").await;
        let dev_sub = dev.subscribe("addressed_to_me").await;

        let status = designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"id": 203, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect("pass_desk_check signal should succeed");
        let status_str = status.as_str().unwrap_or("");
        assert!(
            status_str.contains("signal sent"),
            "pass_desk_check Ok status should report signal sent. got: {status_str}"
        );

        let owner_event = owner
            .wait_for_event(&owner_sub, WAIT_PRESENCE_TIMEOUT)
            .await;
        assert!(
            owner_event.contains("ready for acceptance"),
            "owner must receive the pass signal on the live addressed_to_me push; got {owner_event}"
        );

        let dev_event = dev
            .wait_for_event(&dev_sub, Duration::from_millis(1500))
            .await;
        assert!(
            !dev_event.contains("ready for acceptance"),
            "dev should NOT receive the owner-addressed pass signal; got {dev_event}"
        );

        designer.close().await;
        owner.close().await;
        dev.close().await;
    });
}

#[test]
fn fail_desk_check_signal_reaches_dev_and_the_owner_but_not_other_peers() {
    use harness::mock_gh::{issue_json, mock_story_mutations};
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(203, "fail-desk-check-target", &["desk-check"], None);
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue.clone()]).to_string());
    });
    mock_story_mutations(&server, 203, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect_with_mock_gh(&bpmux, "designer", &base_url).await;
        let owner = TestBridge::connect_with_mock_gh(&bpmux, "owner", &base_url).await;
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let dev_sub = dev.subscribe("addressed_to_me").await;
        let owner_sub = owner.subscribe("addressed_to_me").await;
        let pm_sub = pm.subscribe("addressed_to_me").await;

        let status = designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({"id": 203, "reason": "missing negative test"}),
            )
            .await
            .expect("fail_desk_check signal should succeed");
        let status_str = status.as_str().unwrap_or("");
        assert!(
            status_str.contains("signal sent"),
            "fail_desk_check Ok status should report signal sent. got: {status_str}"
        );

        let dev_event = dev.wait_for_event(&dev_sub, WAIT_PRESENCE_TIMEOUT).await;
        assert!(
            dev_event.contains("missing negative test"),
            "dev (addressed) must receive the fail signal; got {dev_event}"
        );
        let owner_event = owner.wait_for_event(&owner_sub, WAIT_PRESENCE_TIMEOUT).await;
        assert!(
            owner_event.contains("missing negative test"),
            "owner (the omniscient owner) must receive the fail signal; got {owner_event}"
        );

        let pm_event = pm.wait_for_event(&pm_sub, Duration::from_millis(1500)).await;
        assert!(
            !pm_event.contains("missing negative test"),
            "#1733: the owner is unfilterable — dev (addressed) AND owner (the omniscient owner) both receive the fail signal, but pm (a non-owner not in --to) must NOT — owner-omniscience does not loosen peer scoping. got:\n{pm_event}"
        );

        designer.close().await;
        owner.close().await;
        dev.close().await;
        pm.close().await;
    });
}

/// AC (#203): non-designer roles cannot pass_desk_check. The lane
/// enforcement returns an error with a clear message; no signal is
/// fired to anyone.
#[test]
fn pass_desk_check_rejects_non_designer_callers() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        let owner = TestBridge::connect(&bpmux, "owner").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let owner_sub = owner.subscribe("addressed_to_me").await;

        let err = dev
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"id": 203, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect_err("dev should be rejected from pass_desk_check");
        assert!(
            err.contains("workflow-role-not-allowed") && err.contains("\"actor_role\":\"dev\""),
            "rejection cites structured workflow-role-not-allowed kind; got {err}"
        );

        let owner_event = owner
            .wait_for_event(&owner_sub, Duration::from_millis(1500))
            .await;
        assert!(
            !owner_event.contains("ready for acceptance"),
            "rejected pass call should not fire the owner signal; got {owner_event}"
        );

        dev.close().await;
        owner.close().await;
    });
}

#[test]
fn accept_story_allows_pm_and_broadcasts_role_attribution() {
    use harness::mock_gh::{issue_json, mock_story_full};
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(213, "pm-accept-target", &["delivered"], None);
    mock_story_full(&server, 213, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
        let designer = TestBridge::connect_with_mock_gh(&bpmux, "designer", &base_url).await;
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_id": 213,
                    "verdict": "verified",
                    "substrate_grounding": "test-harness: designer verdict"
                }),
            )
            .await
            .expect("designer submit_verdict");
        dev.call_tool(
            "submit_verdict",
            serde_json::json!({
                "story_id": 213,
                "verdict": "verified",
                "substrate_grounding": "test-harness: dev verdict"
            }),
        )
        .await
        .expect("dev submit_verdict");

        pm.call_tool(
            "label_story",
            serde_json::json!({
                "id": 213,
                "label": "awaiting team",
                "rationale": "test fixture: acceptor=team for team-verifiable accept"
            }),
        )
        .await
        .expect("pm label_story awaiting team");

        let dev_sub = dev.subscribe("addressed_to_me").await;

        let result = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"id": 213, "would_file_bug": false}),
            )
            .await;
        assert!(
            result.is_ok(),
            "PM should be allowed to call accept_story. got: {:?}",
            result
        );

        let dev_event = dev.wait_for_event(&dev_sub, WAIT_PRESENCE_TIMEOUT).await;
        assert!(
            dev_event.contains("#213") && dev_event.contains("accepted by pm"),
            "signal should include story number and role. got:\n{dev_event}"
        );

        pm.close().await;
        dev.close().await;
    });
}

/// AC (#213): unknown roles are rejected with a clear message.
#[test]
fn accept_story_rejects_unknown_roles() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let unknown = TestBridge::connect(&bpmux, "intern").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = unknown
            .call_tool(
                "accept_story",
                serde_json::json!({"id": 999, "would_file_bug": false}),
            )
            .await
            .expect_err("unknown role should be rejected");
        assert!(
            err.contains("not a known role") || err.contains("expected one of"),
            "rejection should cite role-vocabulary. got: {err}"
        );

        unknown.close().await;
    });
}

/// #1385 slice 2: designer + dev firing accept_story are substrate-refused
/// per workflow.toml actor_roles (supersedes #213's permissive allowlist).
#[test]
fn accept_story_refuses_designer_and_dev_per_workflow_substrate() {
    use harness::mock_gh::{issue_json, mock_story_mutations};
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue_900 = issue_json(900, "accept-target-1", &["delivered"], None);
    let issue_901 = issue_json(901, "accept-target-2", &["delivered"], None);
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue_900.clone(), issue_901.clone()]).to_string());
    });
    mock_story_mutations(&server, 900, &issue_900);
    mock_story_mutations(&server, 901, &issue_901);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect_with_mock_gh(&bpmux, "designer", &base_url).await;
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = designer
            .call_tool(
                "accept_story",
                serde_json::json!({"id": 900, "would_file_bug": false}),
            )
            .await
            .expect_err("designer should be substrate-refused per workflow.toml");
        assert!(
            err.contains("workflow-role-not-allowed")
                && err.contains("\"actor_role\":\"designer\""),
            "designer rejection cites structured kind + actor; got {err}"
        );

        let err = dev
            .call_tool(
                "accept_story",
                serde_json::json!({"id": 901, "would_file_bug": false}),
            )
            .await
            .expect_err("dev should be substrate-refused per workflow.toml");
        assert!(
            err.contains("workflow-role-not-allowed") && err.contains("\"actor_role\":\"dev\""),
            "dev rejection cites structured kind + actor; got {err}"
        );

        designer.close().await;
        dev.close().await;
    });
}

/// AC (#213): accepting an already-accepted story is a no-op
/// with a clear message, not a raw gh error.
#[test]
fn accept_story_is_idempotent_on_already_closed() {
    use harness::mock_gh::{issue_json, mock_story_full};
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(213, "accept-idempotent-target", &["delivered"], None);
    mock_story_full(&server, 213, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        harness::story_helpers::seed_team_verifiable_evidence_with_mock_gh(&bpmux, &base_url, 213)
            .await;

        let first = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"id": 213, "would_file_bug": false}),
            )
            .await;
        assert!(
            first.is_ok(),
            "first accept_story should succeed. got: {:?}",
            first
        );

        let second = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"id": 213, "would_file_bug": false}),
            )
            .await;
        let err = second.expect_err(
            "second accept_story on already-accepted story should refuse via substrate-validator",
        );
        assert!(
            err.to_string().contains("workflow-target-state-invalid"),
            "second fire must surface workflow-target-state-invalid; got {err}"
        );

        pm.close().await;
    });
}

/// AC (#209a): view_velocity returns JSON metrics parsed from the
/// session log. With a fresh session and no delivered stories, the
/// metrics are zero/sparse — proves the compute path runs end-to-end
/// through the bridge, not just the unit tests.
#[test]
fn view_velocity_returns_metrics_with_sparse_flag_on_empty_session() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let metrics = dev
            .call_tool("view_velocity", serde_json::json!({}))
            .await
            .expect("view_velocity should succeed");

        let body = serde_json::to_string(&metrics).expect("serialize");
        assert!(
            body.contains("\"delivered_in_window\""),
            "metrics should expose delivered_in_window. got: {body}"
        );
        assert!(
            body.contains("\"sparse\": true") || body.contains("\"sparse\":true"),
            "fresh session has no deliveries — sparse should be true. got: {body}"
        );
        assert!(
            body.contains("\"window_days\""),
            "metrics should expose window_days. got: {body}"
        );

        dev.close().await;
    });
}
