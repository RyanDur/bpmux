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
/// via the agent-side path.
///
/// The implemented filter runs on the WebSocket push path (topic_subs
/// in ws.rs). A non-subscribed agent's NotificationCapture therefore
/// proves the negative. The REST read_messages path returns the full
/// curated log regardless of caller identity — a separate gap, out of
/// #199's agent-assertion scope.
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

        // deliver_story fires the signals BEFORE it calls gh to edit
        // labels. The signal routing is what we're verifying; the gh
        // edit is an out-of-scope side effect that fails in CI (no
        // GH_TOKEN) and even locally on stale issue numbers. Accept
        // both outcomes — what matters is the capture assertions
        // below.
        let _ = dev
            .call_tool(
                "deliver_story",
                serde_json::json!({"number": 199, "title": "test api agent assertions"}),
            )
            .await;

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
