#![allow(clippy::panic)]

use crate::bridge::tools_stories::{
    build_accept_signal, build_delivery_signals, build_fail_desk_check_signal,
    build_pass_desk_check_pm_present_signal, build_pass_desk_check_signal, build_reject_signal,
    build_start_signal,
};
use crate::transport::protocol::ClientMessage;

fn extract_text(msg: &ClientMessage) -> &str {
    match msg {
        ClientMessage::Send { text, .. } => text.as_str(),
        _ => panic!("expected ClientMessage::Send"),
    }
}

const IMPERATIVE_VOCABULARY: &[&str] = &[
    "pick the next story",
    "pick the next",
    "fix and re-deliver",
    "fix and redeliver",
    "present to ryan",
    "present to pm",
    "check ready count",
    "check if restocking",
    "check restocking",
    "needs fixes",
    "you may proceed",
    "you should",
    "please ",
];

fn assert_no_imperative(label: &str, text: &str) {
    let lower = text.to_lowercase();
    for phrase in IMPERATIVE_VOCABULARY {
        assert!(
            !lower.contains(phrase),
            "{label}: text {text:?} contains imperative phrase {phrase:?}"
        );
    }
}

#[test]
fn accept_signal_drops_pick_next_story_imperative() {
    let msg = build_accept_signal(123, "pm");
    let text = extract_text(&msg);
    assert!(text.contains("#123"));
    assert!(text.contains("accepted by pm"));
    assert_no_imperative("accept", text);
}

#[test]
fn reject_signal_drops_fix_and_redeliver_imperative() {
    let msg = build_reject_signal(123, "missing test coverage");
    let text = extract_text(&msg);
    assert!(text.contains("#123"));
    assert!(text.contains("rejected"));
    assert!(text.contains("missing test coverage"));
    assert_no_imperative("reject", text);
}

#[test]
fn delivery_signals_drop_check_queue_imperative() {
    let signals = build_delivery_signals(123, "story title");
    assert_eq!(signals.len(), 2);
    for msg in &signals {
        let text = extract_text(msg);
        assert!(text.contains("#123"));
        assert!(text.contains("delivered"));
        assert_no_imperative("delivery", text);
    }
}

#[test]
fn start_signal_drops_check_restocking_imperative() {
    let msg = build_start_signal(123);
    let text = extract_text(&msg);
    assert!(text.contains("#123"));
    assert!(text.contains("started"));
    assert_no_imperative("start", text);
}

#[test]
fn pass_desk_check_pm_present_signal_drops_present_to_ryan_imperative() {
    let msg = build_pass_desk_check_pm_present_signal(123);
    let text = extract_text(&msg);
    assert!(text.contains("#123"));
    assert!(text.contains("passed designer desk-check"));
    assert_no_imperative("pass_desk_check_pm_present", text);
}

#[test]
fn pass_desk_check_signal_is_informational() {
    let msg = build_pass_desk_check_signal(123, "ryan");
    let text = extract_text(&msg);
    assert!(text.contains("#123"));
    assert!(text.contains("passed designer desk-check"));
    assert_no_imperative("pass_desk_check", text);
}

#[test]
fn fail_desk_check_signal_drops_needs_fixes_imperative() {
    let msg = build_fail_desk_check_signal(123, "stub assertion");
    let text = extract_text(&msg);
    assert!(text.contains("#123"));
    assert!(text.contains("failed desk-check"));
    assert!(text.contains("stub assertion"));
    assert_no_imperative("fail_desk_check", text);
}
