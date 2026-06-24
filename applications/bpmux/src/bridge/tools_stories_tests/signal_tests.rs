#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::board::domain::StoryType;
use crate::board::domain::story_type_from_labels_or_title;
use crate::transport::protocol::ClientMessage;

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
fn verdict_reprompt_signal_addresses_expired_role_as_system() {
    if let ClientMessage::Send {
        to,
        addressee,
        message_type,
        text,
    } = build_verdict_reprompt_signal(1599, "designer")
    {
        assert_eq!(to.as_ref().unwrap(), &["designer"]);
        assert_eq!(addressee.as_deref(), Some("designer"));
        assert_eq!(
            message_type.as_deref(),
            Some("system"),
            "a reprompt is a system nudge, not a chat line"
        );
        assert!(
            text.contains("#1599") && text.contains("submit_verdict"),
            "reprompt must name the story and the recovery action; got {text}"
        );
    } else {
        panic!("expected ClientMessage::Send");
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

#[test]
fn pass_desk_check_signal_addresses_owner_for_feature_label() {
    let addressee = addressee_for_pass_desk_check(StoryType::Feature, &[]);
    if let ClientMessage::Send {
        to, addressee: a, ..
    } = build_pass_desk_check_signal(203, addressee)
    {
        assert_eq!(to.as_ref().unwrap(), &["owner"]);
        assert_eq!(a.as_deref(), Some("owner"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn pass_desk_check_signal_addresses_pm_for_chore_label() {
    let addressee = addressee_for_pass_desk_check(StoryType::Chore, &[]);
    if let ClientMessage::Send {
        to, addressee: a, ..
    } = build_pass_desk_check_signal(203, addressee)
    {
        assert_eq!(to.as_ref().unwrap(), &["pm"]);
        assert_eq!(a.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn pass_desk_check_signal_addresses_pm_for_bug_label() {
    let addressee = addressee_for_pass_desk_check(StoryType::Bug, &[]);
    assert_eq!(addressee, "pm");
}

#[test]
fn pass_desk_check_signal_falls_back_to_title_prefix() {
    let addressee = addressee_for_pass_desk_check(
        story_type_from_labels_or_title(&[], "[Chore] split app_state"),
        &[],
    );
    assert_eq!(addressee, "pm");
    let addressee = addressee_for_pass_desk_check(
        story_type_from_labels_or_title(&[], "[Feature] new view"),
        &[],
    );
    assert_eq!(addressee, "owner");
}

#[test]
fn pass_desk_check_signal_defaults_owner_on_empty() {
    let addressee = addressee_for_pass_desk_check(story_type_from_labels_or_title(&[], ""), &[]);
    assert_eq!(addressee, "owner");
    let addressee = addressee_for_pass_desk_check(
        story_type_from_labels_or_title(&["ready".to_string()], "unlabeled story"),
        &[],
    );
    assert_eq!(addressee, "owner");
}

#[test]
fn pass_desk_check_signal_acceptor_label_overrides_title_prefix_team() {
    let labels = vec!["awaiting team".to_string()];
    assert_eq!(
        addressee_for_pass_desk_check(StoryType::Feature, &labels),
        "pm",
        "awaiting team label routes Feature to pm regardless of title-prefix"
    );
    assert_eq!(
        addressee_for_pass_desk_check(StoryType::Epic, &labels),
        "pm"
    );
}

#[test]
fn pass_desk_check_signal_acceptor_label_overrides_title_prefix_owner() {
    let labels = vec!["awaiting owner".to_string()];
    assert_eq!(
        addressee_for_pass_desk_check(StoryType::Bug, &labels),
        "owner",
        "awaiting owner label routes Bug to owner regardless of title-prefix"
    );
    assert_eq!(
        addressee_for_pass_desk_check(StoryType::Chore, &labels),
        "owner"
    );
}

#[test]
fn pass_desk_check_signal_acceptor_label_case_insensitive() {
    let labels = vec!["Awaiting Team".to_string()];
    assert_eq!(
        addressee_for_pass_desk_check(StoryType::Feature, &labels),
        "pm"
    );
}

#[test]
fn pass_desk_check_signal_does_not_target_dev() {
    let addressee = addressee_for_pass_desk_check(story_type_from_labels_or_title(&[], ""), &[]);
    if let ClientMessage::Send { to, .. } = build_pass_desk_check_signal(203, addressee) {
        let recipients = to.as_ref().unwrap();
        assert!(
            !recipients.contains(&"dev".to_string()),
            "pass signal should not go to dev, got: {recipients:?}"
        );
    }
}

#[test]
fn pass_desk_check_signal_announces_ready_for_acceptance() {
    if let ClientMessage::Send { text, .. } = build_pass_desk_check_signal(203, "owner") {
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
fn pass_desk_check_signals_emits_two_for_feature() {
    let signals = build_pass_desk_check_signals(210, StoryType::Feature, &[]);
    assert_eq!(signals.len(), 2, "feature PASS should emit two signals");
    let addressees: Vec<Option<String>> = signals
        .iter()
        .map(|m| match m {
            ClientMessage::Send { addressee, .. } => addressee.clone(),
            _ => None,
        })
        .collect();
    assert!(
        addressees.contains(&Some("owner".to_string())),
        "should address owner: {addressees:?}"
    );
    assert!(
        addressees.contains(&Some("pm".to_string())),
        "should address pm: {addressees:?}"
    );
}

#[test]
fn pass_desk_check_signals_emits_one_for_infra() {
    let signals = build_pass_desk_check_signals(253, StoryType::Chore, &[]);
    assert_eq!(signals.len(), 1, "infra PASS should emit one signal");
    if let ClientMessage::Send { addressee, .. } = &signals[0] {
        assert_eq!(addressee.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn pass_desk_check_signals_ambiguous_routes_to_owner_and_pm() {
    let signals = build_pass_desk_check_signals(999, story_type_from_labels_or_title(&[], ""), &[]);
    assert_eq!(signals.len(), 2);
}

#[test]
fn pass_desk_check_pm_present_signal_targets_pm_with_passed_text() {
    if let ClientMessage::Send { text, to, .. } = build_pass_desk_check_pm_present_signal(210) {
        assert!(text.contains("#210"), "story number in text: {text}");
        assert!(
            text.contains("passed designer desk-check"),
            "PM signal should state desk-check passed, got: {text}"
        );
        assert_eq!(to.as_ref().unwrap(), &["pm"]);
    } else {
        panic!("expected Send variant");
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
fn fail_desk_check_signal_does_not_target_owner() {
    if let ClientMessage::Send { to, .. } = build_fail_desk_check_signal(203, "reason") {
        let recipients = to.as_ref().unwrap();
        assert!(
            !recipients.contains(&"owner".to_string()),
            "fail signal should not go to owner, got: {recipients:?}"
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
            text.contains("failed desk-check"),
            "fail signal should state failed desk-check: {text}"
        );
    }
}
