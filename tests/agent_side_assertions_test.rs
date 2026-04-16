//! Agent-side assertion tests for story #199.
//!
//! These verify what an MCP agent RECEIVES through the bridge's tool
//! responses — closing the two-audience testing gap. The existing
//! harness supports human-side assertions (screen capture via
//! `TestBpmux`); these tests prove the symmetric agent-side path
//! works via `TestBridge::call_tool`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC: "Given a test calls view_board as a specific agent role,
/// when stories exist in the board, then the response contains
/// stories with number, title, and state — matching what that agent
/// would see via MCP."
///
/// Verifies that `call_tool` parses the JSON response and exposes
/// typed fields for assertion. Uses the BPMUX_BOARD_FIXTURE env var
/// to seed deterministic stories without a live gh dependency.
#[test]
fn view_board_response_exposes_story_fields_for_agent_assertions() {
    use std::io::Write;
    let mut fixture = tempfile::NamedTempFile::new().expect("fixture file");
    fixture
        .write_all(
            br#"[
              {"number":100,"title":"Ready one","state":"OPEN","labels":[{"name":"ready"}]},
              {"number":101,"title":"Delivered one","state":"OPEN","labels":[{"name":"delivered"}]}
            ]"#,
        )
        .expect("write fixture");
    let fixture_path = fixture
        .path()
        .to_str()
        .expect("fixture path utf-8")
        .to_string();
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        // Point the bridge's view_board at the fixture rather than the
        // live gh CLI — the MCP tool honors BPMUX_BOARD_FIXTURE the
        // same way the TUI does. Keeps the test hermetic (no GH_TOKEN
        // required in CI).
        let dev =
            TestBridge::connect_with_env(&bpmux, "dev", &[("BPMUX_BOARD_FIXTURE", &fixture_path)])
                .await;
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
            rendered.contains("READY (") && rendered.contains("DELIVERED ("),
            "view_board should group stories by state. got:\n{rendered}"
        );
        dev.close().await;
    });
}

/// AC: non-participant of a topic does NOT receive the topic message
/// via the agent-side push path.
///
/// The filter runs on the WebSocket push path via `topic_subs` in
/// ws.rs. #204 closed the matching gap on the REST read_messages
/// pull path — see `read_messages_filters_topic_messages_for_non_subscribed_agent`
/// below for that assertion.
///
/// Subscribes pm to "teaching" (auto, on first topic send). dev
/// stays on broadcast only. Asserts dev's capture contains the
/// broadcast but not the topic content.
#[test]
fn topic_messages_are_filtered_from_non_subscribed_agents_capture() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;

        // Allow both bridges to finish WebSocket handshake + join
        // announcements before we send anything.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Plain broadcast hits everyone — syncs setup.
        pm.send("plain broadcast").await;
        dev.capture
            .wait_for("plain broadcast", Duration::from_secs(5))
            .await;

        // Topic message: pm auto-subscribes to "teaching" on send;
        // dev is not a participant.
        pm.call_tool(
            "send_message",
            serde_json::json!({"text": "secret topic content", "topic": "teaching"}),
        )
        .await
        .expect("topic send");

        // Send a second broadcast so there's a known-delivered event
        // after the topic message; by the time dev sees it, the topic
        // message would have arrived too if the filter were broken.
        pm.send("barrier broadcast").await;
        dev.capture
            .wait_for("barrier broadcast", Duration::from_secs(5))
            .await;

        let dev_seen = dev.capture.received().join("\n");
        assert!(
            dev_seen.contains("plain broadcast") && dev_seen.contains("barrier broadcast"),
            "dev should see broadcasts. got:\n{dev_seen}"
        );
        assert!(
            !dev_seen.contains("secret topic content"),
            "dev (non-participant) should NOT see topic messages. got:\n{dev_seen}"
        );

        pm.close().await;
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
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;

        // Wait for both bridges to finish their join announcements
        // before the signal fires, so the capture windows are stable.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // deliver_story's API returns Err only on signal-send failure
        // (load-bearing). gh label edits are reported in the Ok
        // status string, so the test asserts that signals succeeded
        // (Ok) without masking the gh outcome. This removes the
        // earlier `let _ =` variance tolerance — the test now takes
        // one execution path regardless of whether gh is available.
        let status = dev
            .call_tool(
                "deliver_story",
                serde_json::json!({"number": 199, "title": "test api agent assertions"}),
            )
            .await
            .expect("deliver_story signals should succeed");
        let status_str = status.as_str().unwrap_or("");
        assert!(
            status_str.contains("signals sent"),
            "deliver_story Ok status should report signals sent. got: {status_str}"
        );

        // Designer receives the desk-check addressed signal.
        designer
            .capture
            .wait_for("desk-check needed", Duration::from_secs(5))
            .await;

        // Negative: dev does NOT receive the desk-check signal. Give
        // the broadcast time to propagate, then snapshot dev's
        // captured notifications and assert the phrase is absent.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let dev_seen = dev.capture.received().join("\n");
        assert!(
            !dev_seen.contains("desk-check needed"),
            "dev should NOT receive the designer-addressed desk-check signal. got:\n{dev_seen}"
        );

        dev.close().await;
        designer.close().await;
    });
}

/// AC (#203): designer calling pass_desk_check fires a signal
/// addressed to ryan. dev's notification stream does NOT contain
/// the signal — routing is exact, not broadcast.
#[test]
fn pass_desk_check_signal_routes_to_ryan_not_dev() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect(&bpmux, "designer").await;
        let ryan = TestBridge::connect(&bpmux, "ryan").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let status = designer
            .call_tool("pass_desk_check", serde_json::json!({"number": 203}))
            .await
            .expect("pass_desk_check signal should succeed");
        let status_str = status.as_str().unwrap_or("");
        assert!(
            status_str.contains("signal sent"),
            "pass_desk_check Ok status should report signal sent. got: {status_str}"
        );

        ryan.capture
            .wait_for("ready for acceptance", Duration::from_secs(5))
            .await;

        // Negative: dev does not see the pass signal. Wait a beat for
        // any mis-routed broadcast, then snapshot.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let dev_seen = dev.capture.received().join("\n");
        assert!(
            !dev_seen.contains("ready for acceptance"),
            "dev should NOT receive the ryan-addressed pass signal. got:\n{dev_seen}"
        );

        designer.close().await;
        ryan.close().await;
        dev.close().await;
    });
}

/// AC (#203): designer calling fail_desk_check fires a signal
/// addressed to dev with the reason. ryan's notification stream
/// does NOT contain the signal.
#[test]
fn fail_desk_check_signal_routes_to_dev_not_ryan() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect(&bpmux, "designer").await;
        let ryan = TestBridge::connect(&bpmux, "ryan").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let status = designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({"number": 203, "reason": "missing negative test"}),
            )
            .await
            .expect("fail_desk_check signal should succeed");
        let status_str = status.as_str().unwrap_or("");
        assert!(
            status_str.contains("signal sent"),
            "fail_desk_check Ok status should report signal sent. got: {status_str}"
        );

        dev.capture
            .wait_for("missing negative test", Duration::from_secs(5))
            .await;

        tokio::time::sleep(Duration::from_millis(500)).await;
        let ryan_seen = ryan.capture.received().join("\n");
        assert!(
            !ryan_seen.contains("missing negative test"),
            "ryan should NOT receive the dev-addressed fail signal. got:\n{ryan_seen}"
        );

        designer.close().await;
        ryan.close().await;
        dev.close().await;
    });
}

/// AC (#204): non-participant of a topic does NOT see the topic
/// message via read_messages (the REST pull path), matching the
/// WebSocket push filter. Before this fix the two paths diverged —
/// an agent polling via read_messages could see topic messages it
/// wasn't subscribed to, while the same agent receiving via push
/// could not.
#[test]
fn read_messages_filters_topic_messages_for_non_subscribed_agent() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;

        // Let both bridges finish WebSocket handshake + join
        // announcements before sending.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Broadcast to confirm the baseline pull path works for both.
        pm.send("plain broadcast").await;

        // pm auto-subscribes to "teaching" on first topic send; dev
        // stays on broadcast only.
        pm.call_tool(
            "send_message",
            serde_json::json!({"text": "secret topic content", "topic": "teaching"}),
        )
        .await
        .expect("topic send");

        // Barrier broadcast gives us a known-delivered tail event —
        // dev's capture passes this barrier before we read.
        pm.send("barrier broadcast").await;
        dev.capture
            .wait_for("barrier broadcast", Duration::from_secs(5))
            .await;

        let dev_messages = dev
            .call_tool("read_messages", serde_json::json!({}))
            .await
            .expect("read_messages");
        let body = serde_json::to_string(&dev_messages).expect("serialize");
        assert!(
            body.contains("plain broadcast") && body.contains("barrier broadcast"),
            "dev should see broadcasts via read_messages. got: {body}"
        );
        assert!(
            !body.contains("secret topic content"),
            "dev (non-participant) should NOT see topic messages via read_messages. got: {body}"
        );

        pm.close().await;
        dev.close().await;
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
        let ryan = TestBridge::connect(&bpmux, "ryan").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("pass_desk_check", serde_json::json!({"number": 203}))
            .await
            .expect_err("dev should be rejected from pass_desk_check");
        assert!(
            err.contains("restricted to designer"),
            "rejection message should name the required role. got: {err}"
        );

        tokio::time::sleep(Duration::from_millis(500)).await;
        let ryan_seen = ryan.capture.received().join("\n");
        assert!(
            !ryan_seen.contains("ready for acceptance"),
            "rejected pass call should not fire the ryan signal. got:\n{ryan_seen}"
        );

        dev.close().await;
        ryan.close().await;
    });
}

/// AC (#213): PM can call accept_story. The signal broadcasts
/// "accepted by pm" so Ryan sees who closed the story.
#[test]
fn accept_story_allows_pm_and_broadcasts_role_attribution() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("accept_story", serde_json::json!({"number": 213}))
            .await;
        // Role gate must pass — reject only if it's a role-restriction
        // error. gh failures (no GH_TOKEN in CI) are expected and don't
        // indicate a role-gate problem.
        if let Err(ref e) = result {
            assert!(
                !e.contains("restricted to"),
                "PM should pass the role gate. got: {e}"
            );
        }

        // Signal fires on intent before gh — dev should see it even
        // when gh fails.
        dev.capture
            .wait_for("accepted by pm", Duration::from_secs(5))
            .await;

        let dev_seen = dev.capture.received().join("\n");
        assert!(
            dev_seen.contains("#213") && dev_seen.contains("accepted by pm"),
            "signal should include story number and role. got:\n{dev_seen}"
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
            .call_tool("accept_story", serde_json::json!({"number": 999}))
            .await
            .expect_err("unknown role should be rejected");
        assert!(
            err.contains("restricted to"),
            "rejection should name allowed roles. got: {err}"
        );

        unknown.close().await;
    });
}

/// AC (#213): designer and dev pass the role gate (not rejected).
/// gh may fail on non-existent issue numbers — that's expected in
/// test; the assertion is that the error is NOT a role-restriction.
#[test]
fn accept_story_allows_designer_and_dev() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect(&bpmux, "designer").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = designer
            .call_tool("accept_story", serde_json::json!({"number": 900}))
            .await;
        if let Err(ref e) = result {
            assert!(
                !e.contains("restricted to"),
                "designer should pass the role gate. got: {e}"
            );
        }

        let result = dev
            .call_tool("accept_story", serde_json::json!({"number": 901}))
            .await;
        if let Err(ref e) = result {
            assert!(
                !e.contains("restricted to"),
                "dev should pass the role gate. got: {e}"
            );
        }

        designer.close().await;
        dev.close().await;
    });
}

/// AC (#213): accepting an already-accepted story is a no-op
/// with a clear message, not a raw gh error.
#[test]
fn accept_story_is_idempotent_on_already_closed() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        // First accept
        let _ = pm
            .call_tool("accept_story", serde_json::json!({"number": 213}))
            .await;

        // Second accept — should be no-op, not error
        let result = pm
            .call_tool("accept_story", serde_json::json!({"number": 213}))
            .await;
        assert!(
            result.is_ok(),
            "double-accept should be a no-op, not an error. got: {:?}",
            result
        );
        let msg = result.unwrap();
        let text = serde_json::to_string(&msg).unwrap_or_default();
        assert!(
            text.contains("already") || text.contains("accepted"),
            "idempotent message should say already accepted. got: {text}"
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
