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
fn require_role_rejects_agents_from_owner_operations() {
    let result = require_role("dev:claude-code", &["ryan"], "accept");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to ryan"));

    let result = require_role("pm:claude-code", &["ryan"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to ryan"));
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
fn accept_signal_targets_dev_only() {
    if let ClientMessage::Send { to, addressee, .. } = build_accept_signal(100) {
        assert_eq!(to.as_ref().unwrap(), &["dev"]);
        assert_eq!(addressee.as_deref(), Some("dev"));
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
