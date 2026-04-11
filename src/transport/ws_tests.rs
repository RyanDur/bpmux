#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::ws::*;
use crate::session;
use crate::session::MessageType;
use chrono::Utc;

fn msg(from: &str, to: Option<Vec<&str>>) -> session::Message {
    session::Message {
        from: from.to_string(),
        text: "test".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: to.map(|v| v.into_iter().map(String::from).collect()),
        addressee: None,
        explicit_privacy: false,
    }
}

#[test]
fn broadcast_delivered_to_everyone() {
    assert!(should_deliver(
        &msg("dev:claude-code", None),
        "pm:claude-code"
    ));
    assert!(should_deliver(&msg("dev:claude-code", None), "ryan:human"));
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
// Message-size regression guard (#tbd — 2026-04-11 silent-drop fix).
//
// Original limit: 4096 bytes. Raised to 65536 (64 KB) after coordination
// messages (test instructions, retrospectives) repeatedly exceeded the
// old limit and were silently dropped. These tests prevent accidental
// re-shrinking of the limit by a future refactor — if someone lowers it
// below what legitimate bpmux messages need, the tests fail loudly at
// build time instead of re-introducing the silent failure mode.
// ─────────────────────────────────────────────────────────────────────

#[test]
fn message_size_ok_accepts_typical_coordination_message() {
    // Typical small coordination message — about the size of a
    // raise-hand ping or a short acknowledgment.
    let typical = "pm: 🖐 pm — distinct angle on the sequencing";
    assert!(message_size_ok(typical));
}

#[test]
fn message_size_ok_accepts_multi_paragraph_instruction() {
    // The kind of multi-paragraph nav/drive step that was dropping
    // under the 4096 limit. ~3 KB of structured markdown.
    let instruction = "x".repeat(3 * 1024);
    assert!(
        message_size_ok(&instruction),
        "3 KB coordination messages must land — this was the drop pattern on 2026-04-10"
    );
}

#[test]
fn message_size_ok_accepts_session_retro() {
    // Session retros observed at ~5 KB during the 2026-04-10 session.
    // This test fails if anyone lowers the limit below that threshold,
    // which would silently re-introduce the retro-drop bug.
    let retro = "x".repeat(5 * 1024);
    assert!(
        message_size_ok(&retro),
        "5 KB session retros must land — dropping them is the silent-drop failure mode"
    );
}

#[test]
fn message_size_ok_accepts_comfortable_margin() {
    // 32 KB — halfway between typical use and the 64 KB ceiling. If
    // this starts failing, someone has lowered the ceiling dramatically
    // and should justify it with a design note + a successor approach
    // (e.g., chunked messaging) for legitimate large payloads.
    let generous = "x".repeat(32 * 1024);
    assert!(
        message_size_ok(&generous),
        "32 KB headroom must exist above typical coordination traffic"
    );
}

#[test]
fn message_size_ok_rejects_ddos_sized_payload() {
    // Messages significantly larger than the ceiling should be
    // rejected — the ceiling is a DoS safeguard, not a theoretical
    // limit. 1 MB is the canonical "this is definitely an abuse"
    // payload size.
    let ddos = "x".repeat(1024 * 1024);
    assert!(
        !message_size_ok(&ddos),
        "1 MB messages must be rejected — the size cap is a DoS safeguard"
    );
}
