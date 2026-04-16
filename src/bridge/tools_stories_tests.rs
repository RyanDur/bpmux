#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::tools_stories::*;
use crate::transport::protocol::ClientMessage;

#[test]
fn role_from_name_extracts_short_role() {
    assert_eq!(role_from_name("dev:claude-code"), "dev");
    assert_eq!(role_from_name("pm"), "pm");
    assert_eq!(role_from_name("designer:cursor"), "designer");
    assert_eq!(role_from_name("ryan:human"), "ryan");
}

#[test]
fn require_role_allows_matching_role() {
    assert!(require_role("dev:claude-code", &["dev"], "start").is_ok());
    assert!(require_role("pm", &["pm"], "label").is_ok());
    assert!(require_role("ryan:human", &["ryan"], "accept").is_ok());
}

#[test]
fn require_role_rejects_non_matching_role() {
    let result = require_role("designer:claude-code", &["dev"], "start");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to dev"));
}

#[test]
fn require_role_rejects_pm_from_dev_operations() {
    let result = require_role("pm:claude-code", &["dev"], "deliver");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to dev"));
}

#[test]
fn require_role_rejects_dev_from_pm_operations() {
    let result = require_role("dev:claude-code", &["pm"], "label");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[test]
fn require_role_rejects_agents_from_owner_only_operations() {
    let result = require_role("dev:claude-code", &["ryan"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to ryan"));

    let result = require_role("pm:claude-code", &["ryan"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to ryan"));
}

#[test]
fn require_role_allows_designer_for_desk_check_operations() {
    assert!(require_role("designer:claude-code", &["designer"], "pass desk-check").is_ok());
    assert!(require_role("designer", &["designer"], "fail desk-check").is_ok());
}

#[test]
fn require_role_rejects_non_designer_from_desk_check_operations() {
    for caller in ["dev:claude-code", "pm:claude-code", "ryan:human"] {
        let result = require_role(caller, &["designer"], "pass desk-check");
        assert!(
            result.is_err(),
            "{caller} should not pass desk-check; got: {result:?}"
        );
        assert!(result.unwrap_err().contains("restricted to designer"));
    }
}

// ── Workflow signal tests (#197) ──

#[test]
fn delivery_signals_target_designer_and_pm() {
    let signals = build_delivery_signals(191, "topic channels");
    assert_eq!(signals.len(), 2, "deliver should produce 2 signals");

    if let ClientMessage::Send { to, addressee, .. } = &signals[0] {
        assert_eq!(to.as_ref().unwrap(), &["designer"]);
        assert_eq!(addressee.as_deref(), Some("designer"));
    } else {
        panic!("expected Send variant");
    }

    if let ClientMessage::Send { to, addressee, .. } = &signals[1] {
        assert_eq!(to.as_ref().unwrap(), &["pm"]);
        assert_eq!(addressee.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn delivery_signals_do_not_target_dev() {
    let signals = build_delivery_signals(191, "topic channels");
    for signal in &signals {
        if let ClientMessage::Send { to, .. } = signal {
            let recipients = to.as_ref().unwrap();
            assert!(
                !recipients.contains(&"dev".to_string()),
                "delivery signal should NOT target dev, got: {recipients:?}"
            );
        }
    }
}

#[test]
fn start_signal_targets_pm_only() {
    if let ClientMessage::Send { to, addressee, .. } = build_start_signal(100) {
        assert_eq!(to.as_ref().unwrap(), &["pm"]);
        assert_eq!(addressee.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn accept_signal_broadcasts_with_role_attribution() {
    if let ClientMessage::Send {
        to,
        addressee,
        text,
        ..
    } = build_accept_signal(100, "pm")
    {
        assert!(
            to.is_none(),
            "accept signal should broadcast (no privacy filter)"
        );
        assert_eq!(addressee.as_deref(), Some("dev"));
        assert!(
            text.contains("accepted by pm"),
            "signal should include who accepted. got: {text}"
        );
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn reject_signal_targets_dev_only() {
    if let ClientMessage::Send { to, addressee, .. } = build_reject_signal(100, "needs fix") {
        assert_eq!(to.as_ref().unwrap(), &["dev"]);
        assert_eq!(addressee.as_deref(), Some("dev"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn reject_signal_includes_reason() {
    if let ClientMessage::Send { text, .. } = build_reject_signal(42, "missing AC") {
        assert!(
            text.contains("missing AC"),
            "reason should be in text: {text}"
        );
        assert!(
            text.contains("#42"),
            "story number should be in text: {text}"
        );
    }
}

// ── #203 desk-check signal tests ──

#[test]
fn pass_desk_check_signal_targets_ryan_only() {
    if let ClientMessage::Send { to, addressee, .. } = build_pass_desk_check_signal(203) {
        assert_eq!(to.as_ref().unwrap(), &["ryan"]);
        assert_eq!(addressee.as_deref(), Some("ryan"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn pass_desk_check_signal_does_not_target_dev() {
    if let ClientMessage::Send { to, .. } = build_pass_desk_check_signal(203) {
        let recipients = to.as_ref().unwrap();
        assert!(
            !recipients.contains(&"dev".to_string()),
            "pass signal should not go to dev, got: {recipients:?}"
        );
    }
}

#[test]
fn pass_desk_check_signal_announces_ready_for_acceptance() {
    if let ClientMessage::Send { text, .. } = build_pass_desk_check_signal(203) {
        assert!(
            text.contains("#203"),
            "story number should be in text: {text}"
        );
        assert!(
            text.contains("ready for acceptance"),
            "pass signal should say 'ready for acceptance': {text}"
        );
    }
}

#[test]
fn fail_desk_check_signal_targets_dev_only() {
    if let ClientMessage::Send { to, addressee, .. } = build_fail_desk_check_signal(203, "reason") {
        assert_eq!(to.as_ref().unwrap(), &["dev"]);
        assert_eq!(addressee.as_deref(), Some("dev"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn fail_desk_check_signal_does_not_target_ryan() {
    if let ClientMessage::Send { to, .. } = build_fail_desk_check_signal(203, "reason") {
        let recipients = to.as_ref().unwrap();
        assert!(
            !recipients.contains(&"ryan".to_string()),
            "fail signal should not go to ryan, got: {recipients:?}"
        );
    }
}

#[test]
fn fail_desk_check_signal_includes_reason() {
    if let ClientMessage::Send { text, .. } =
        build_fail_desk_check_signal(203, "missing negative case")
    {
        assert!(
            text.contains("missing negative case"),
            "reason should be in text: {text}"
        );
        assert!(
            text.contains("#203"),
            "story number should be in text: {text}"
        );
        assert!(
            text.contains("needs fixes"),
            "fail signal should say 'needs fixes': {text}"
        );
    }
}
