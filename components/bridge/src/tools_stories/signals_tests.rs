#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use crate::tools_stories::{
    build_accept_signal, build_delivery_signals, build_fail_desk_check_signal,
    build_pass_desk_check_pm_present_signal, build_pass_desk_check_signal, build_reject_signal,
    build_start_signal,
};
use protocol::ClientMessage;

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
    "present to owner",
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
fn pass_desk_check_pm_present_signal_drops_present_to_owner_imperative() {
    let msg = build_pass_desk_check_pm_present_signal(123);
    let text = extract_text(&msg);
    assert!(text.contains("#123"));
    assert!(text.contains("passed designer desk-check"));
    assert_no_imperative("pass_desk_check_pm_present", text);
}

#[test]
fn pass_desk_check_signal_is_informational() {
    let msg = build_pass_desk_check_signal(123, "owner");
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

mod routing_and_addressees {

    use crate::tools_stories::*;
    use product_management::story::Acceptor;
    use product_management::story::StoryKind;
    use product_management::story::story_type_from_title;
    use protocol::ClientMessage;

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
            to_ids: None,
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
        let addressee = addressee_for_pass_desk_check(StoryKind::Feature, None);
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
        let addressee = addressee_for_pass_desk_check(StoryKind::Chore, None);
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
        let addressee = addressee_for_pass_desk_check(StoryKind::Bug, None);
        assert_eq!(addressee, "pm");
    }

    #[test]
    fn pass_desk_check_signal_falls_back_to_title_prefix() {
        let addressee =
            addressee_for_pass_desk_check(story_type_from_title("[Chore] split app_state"), None);
        assert_eq!(addressee, "pm");
        let addressee =
            addressee_for_pass_desk_check(story_type_from_title("[Feature] new view"), None);
        assert_eq!(addressee, "owner");
    }

    #[test]
    fn pass_desk_check_signal_defaults_owner_on_empty() {
        let addressee = addressee_for_pass_desk_check(story_type_from_title(""), None);
        assert_eq!(addressee, "owner");
        let addressee =
            addressee_for_pass_desk_check(story_type_from_title("unlabeled story"), None);
        assert_eq!(addressee, "owner");
    }

    #[test]
    fn pass_desk_check_signal_team_acceptor_overrides_title_prefix() {
        assert_eq!(
            addressee_for_pass_desk_check(StoryKind::Feature, Some(Acceptor::Team)),
            "pm",
            "team acceptor routes Feature to pm regardless of title-prefix"
        );
        assert_eq!(
            addressee_for_pass_desk_check(StoryKind::Bug, Some(Acceptor::Team)),
            "pm",
            "team acceptor routes any story kind to pm"
        );
    }

    #[test]
    fn pass_desk_check_signal_owner_acceptor_overrides_title_prefix() {
        assert_eq!(
            addressee_for_pass_desk_check(StoryKind::Bug, Some(Acceptor::Owner)),
            "owner",
            "owner acceptor routes Bug to owner regardless of title-prefix"
        );
        assert_eq!(
            addressee_for_pass_desk_check(StoryKind::Chore, Some(Acceptor::Owner)),
            "owner"
        );
    }

    #[test]
    fn pass_desk_check_signal_does_not_target_dev() {
        let addressee = addressee_for_pass_desk_check(story_type_from_title(""), None);
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
        let signals = build_pass_desk_check_signals(210, StoryKind::Feature, None);
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
        let signals = build_pass_desk_check_signals(253, StoryKind::Chore, None);
        assert_eq!(signals.len(), 1, "infra PASS should emit one signal");
        if let ClientMessage::Send { addressee, .. } = &signals[0] {
            assert_eq!(addressee.as_deref(), Some("pm"));
        } else {
            panic!("expected Send variant");
        }
    }

    #[test]
    fn pass_desk_check_signals_ambiguous_routes_to_owner_and_pm() {
        let signals = build_pass_desk_check_signals(999, story_type_from_title(""), None);
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
        if let ClientMessage::Send { to, addressee, .. } =
            build_fail_desk_check_signal(203, "reason")
        {
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
}
