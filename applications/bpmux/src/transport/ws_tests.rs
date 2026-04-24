#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::ws::*;
use crate::session;
use chrono::Utc;

fn msg(from: &str, to: Option<Vec<&str>>) -> session::Message {
    session::Message {
        from: from.to_string(),
        text: "test".to_string(),
        ts: Utc::now(),
        to: to.map(|v| v.into_iter().map(String::from).collect()),
        ..session::Message::default()
    }
}

#[test]
fn broadcast_delivered_to_everyone() {
    assert!(should_deliver(
        &msg("dev:claude-code", None),
        "pm:claude-code",
    ));
    assert!(should_deliver(&msg("dev:claude-code", None), "ryan:human",));
}

#[test]
fn addressed_delivered_to_recipient() {
    let m = msg("dev:claude-code", Some(vec!["pm"]));
    assert!(should_deliver(&m, "pm:claude-code"));
}

#[test]
fn addressed_delivered_to_sender() {
    let m = msg("dev:claude-code", Some(vec!["pm"]));
    assert!(should_deliver(&m, "dev:claude-code"));
}

#[test]
fn addressed_not_delivered_to_non_recipient() {
    let m = msg("dev:claude-code", Some(vec!["pm"]));
    assert!(!should_deliver(&m, "designer:claude-code"));
}

#[test]
fn group_addressed_delivered_to_all_named() {
    let m = msg("dev:claude-code", Some(vec!["pm", "designer"]));
    assert!(should_deliver(&m, "pm:claude-code"));
    assert!(should_deliver(&m, "designer:claude-code"));
    assert!(should_deliver(&m, "dev:human"));
    assert!(!should_deliver(&m, "ryan:human"));
}

#[test]
fn to_all_is_broadcast() {
    let m = msg("dev:claude-code", Some(vec!["all"]));
    assert!(should_deliver(&m, "pm:claude-code"));
    assert!(should_deliver(&m, "ryan:human"));
    assert!(should_deliver(&m, "designer:claude-code"));
}

#[test]
fn short_name_strips_tool_suffix() {
    assert_eq!(short_name("dev:claude-code"), "dev");
    assert_eq!(short_name("ryan:human"), "ryan");
    assert_eq!(short_name("bpmux"), "bpmux");
}

// ─────────────────────────────────────────────────────────────────────
// `message_size_ok` unit tests — regression guard on the pure size
// check helper (#tbd — 2026-04-11 silent-drop fix).
//
// **Scope**: these are UNIT TESTS on the `message_size_ok` pure
// function. They call the function directly and assert on its return
// value. They do NOT exercise the full WebSocket send path — a message
// of "typical" size landing here means `message_size_ok(text)` returned
// `true`, not "a message sent via the real WS stack was accepted."
//
// **Why unit, not end-to-end**: the size check is a pure function
// extracted specifically for testability (see
// `feedback_confidence_framing_not_correction.md` + the
// success_extract_testable_pure_functions lesson). Unit-testing the
// extracted helper is appropriate. End-to-end coverage of the real
// WebSocket size rejection path would be a separate test that stood
// up a real `axum::Router` with the `handle_client_message` handler
// and sent a large frame via a tungstenite client — that's valuable
// but a distinct scope.
//
// **Rename audit (2026-04-11 evening)**: these tests were originally
// named `message_size_ok_accepts_typical_coordination_message` and
// similar, framing them as system-behavior assertions ("accepts",
// "rejects"). Ryan caught the mismatch: the names promised behavior,
// the content delivered a pure-function check. Renamed to
// `message_size_ok_returns_{true,false}_for_*` so the name matches
// the content — a pure unit test on a pure function. Original
// intent preserved in the doc comments below each test.
// ─────────────────────────────────────────────────────────────────────

#[test]
fn message_size_ok_returns_true_for_short_ping_sized_text() {
    // Original intent: typical small coordination message — about the
    // size of a raise-hand ping or a short acknowledgment. Catches
    // regression if the limit were lowered below ping-size (which
    // would be absurd but the test costs nothing).
    let typical = "pm: 🖐 pm — distinct angle on the sequencing";
    assert!(message_size_ok(typical));
}

#[test]
fn message_size_ok_returns_true_for_3kb_text() {
    // Original intent: the kind of multi-paragraph nav/drive step that
    // was silently dropping under the old 4096 limit. ~3 KB of text.
    // If this starts failing, someone has lowered `MAX_MESSAGE_SIZE`
    // back toward the broken value.
    let text = "x".repeat(3 * 1024);
    assert!(message_size_ok(&text));
}

#[test]
fn message_size_ok_returns_true_for_5kb_text() {
    // Original intent: session retros observed at ~5 KB during the
    // 2026-04-10 session. If this fails, the limit would silently
    // re-introduce the retro-drop failure mode.
    let text = "x".repeat(5 * 1024);
    assert!(message_size_ok(&text));
}

#[test]
fn message_size_ok_returns_true_for_32kb_text() {
    // Original intent: halfway between typical use and the 64 KB
    // ceiling. Guards against dramatic ceiling reductions that would
    // cramp future large payload use cases (code paste, diff dumps).
    let text = "x".repeat(32 * 1024);
    assert!(message_size_ok(&text));
}

#[test]
fn message_size_ok_returns_false_for_1mb_text() {
    // Original intent: messages significantly larger than the ceiling
    // are rejected. 1 MB is the canonical "this is definitely abuse"
    // payload. Guards the DoS safeguard property.
    let text = "x".repeat(1024 * 1024);
    assert!(!message_size_ok(&text));
}

#[test]
fn peer_lag_message_shape_is_system_typed() {
    let ts = Utc::now();
    let msg = build_peer_lag_message("dev:claude-code", 42, ts);
    assert_eq!(msg.from, "bpmux");
    assert_eq!(msg.message_type, session::MessageType::System);
    assert!(msg.text.starts_with(PEER_LAG_MARKER));
}

#[test]
fn peer_lag_message_text_embeds_peer_and_count() {
    let msg = build_peer_lag_message("pm:claude-code", 7, Utc::now());
    assert!(
        msg.text.contains("pm:claude-code"),
        "text missing peer: {}",
        msg.text
    );
    assert!(
        msg.text.contains("\"count\":7"),
        "text missing count: {}",
        msg.text
    );
}

#[test]
fn peer_lag_roundtrip_parses_peer_and_count() {
    let msg = build_peer_lag_message("designer:claude-code", 13, Utc::now());
    let (peer, count) = parse_peer_lag_payload(&msg.text).expect("parse");
    assert_eq!(peer, "designer:claude-code");
    assert_eq!(count, 13);
}

#[test]
fn peer_lag_parse_rejects_non_marker_text() {
    assert!(parse_peer_lag_payload("some other system message").is_none());
    assert!(parse_peer_lag_payload("__BPMUX_BOARD_INVALIDATE__").is_none());
}

#[test]
fn peer_lag_parse_rejects_malformed_json() {
    let malformed = format!("{PEER_LAG_MARKER}:not json");
    assert!(parse_peer_lag_payload(&malformed).is_none());
}

#[test]
fn peer_lag_ts_preserved_on_builder() {
    let ts = Utc::now();
    let msg = build_peer_lag_message("dev", 1, ts);
    assert_eq!(msg.ts, ts);
}
