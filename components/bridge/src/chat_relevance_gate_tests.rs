#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::chat_relevance::{
    addressed_to_me, chat_class_reaches_live_stream, scope_reaches_live_stream,
};

fn classes(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}

fn recipients(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}

#[test]
fn owner_bridge_stream_receives_a_conversation_scoped_to_other_peers() {
    let to = recipients(&["pm", "designer"]);
    assert!(
        scope_reaches_live_stream(Some(&to), None, None, "owner"),
        "#1733-rework: the owner's live-notification stream sees a chat scoped to:[pm,designer] — the bridge scope-gate is unfilterable for the owner, mirroring should_deliver. --to is peer-attention, never owner-hiding"
    );
}

#[test]
fn bridge_scope_gate_does_not_loosen_peer_scoping_for_non_owners() {
    let to = recipients(&["pm", "designer"]);
    assert!(
        !scope_reaches_live_stream(Some(&to), None, None, "dev"),
        "the owner-exemption does NOT loosen scope for peers — a non-owner not named in --to is still excluded at the bridge scope-gate"
    );
}

#[test]
fn bridge_scope_gate_passes_broadcasts_to_every_stream() {
    assert!(scope_reaches_live_stream(None, None, None, "dev"));
    assert!(scope_reaches_live_stream(
        Some(&recipients(&["all"])),
        None,
        None,
        "dev"
    ));
}

#[test]
fn bridge_scope_gate_passes_a_message_naming_the_recipient() {
    assert!(scope_reaches_live_stream(
        Some(&recipients(&["dev"])),
        None,
        None,
        "dev"
    ));
}

#[test]
fn bridge_scope_gate_id_precision_excludes_a_display_sibling() {
    let to = recipients(&["dev"]);
    let to_ids = vec!["id-dev-1".to_string()];
    assert!(
        scope_reaches_live_stream(Some(&to), Some(&to_ids), Some("id-dev-1"), "dev"),
        "id-dev-1 is the frozen to_ids target and must see the scoped stream"
    );
    assert!(
        !scope_reaches_live_stream(Some(&to), Some(&to_ids), Some("id-dev-2"), "dev"),
        "id-dev-2 shares the display 'dev' with id-dev-1 but is a DIFFERENT user — id precision must exclude it from the bridge scope-gate even though the display matches"
    );
}

#[test]
fn bridge_scope_gate_legacy_row_falls_back_to_display() {
    let to = recipients(&["dev"]);
    assert!(scope_reaches_live_stream(
        Some(&to),
        None,
        Some("id-dev-1"),
        "dev"
    ));
    assert!(scope_reaches_live_stream(
        Some(&to),
        None,
        Some("id-dev-2"),
        "dev"
    ));
}

#[test]
fn addressed_to_me_matches_addressee_by_id_and_excludes_a_display_sibling() {
    let addressee_ids = vec!["id-dev-1".to_string()];
    assert!(
        addressed_to_me(
            Some("dev"),
            Some(&addressee_ids),
            None,
            None,
            Some("id-dev-1"),
            "dev"
        ),
        "id-dev-1 is the frozen addressee_ids target"
    );
    assert!(
        !addressed_to_me(
            Some("dev"),
            Some(&addressee_ids),
            None,
            None,
            Some("id-dev-2"),
            "dev"
        ),
        "id-dev-2 shares the display 'dev' with id-dev-1 but is a DIFFERENT user — id precision must exclude it"
    );
}

#[test]
fn addressed_to_me_matches_a_direct_to_id_but_never_a_broadcast() {
    let to = recipients(&["dev"]);
    let to_ids = vec!["id-dev-1".to_string()];
    assert!(
        addressed_to_me(
            None,
            None,
            Some(&to),
            Some(&to_ids),
            Some("id-dev-1"),
            "dev"
        ),
        "a direct to_ids match (no addressee) still counts as personally named"
    );
    let broadcast = recipients(&["all"]);
    assert!(
        !addressed_to_me(
            None,
            None,
            Some(&broadcast),
            Some(&[]),
            Some("id-dev-1"),
            "dev"
        ),
        "a broadcast never counts as personally named, even if the caller's id is known"
    );
}

#[test]
fn addressed_to_me_legacy_row_falls_back_to_display_for_both_addressee_and_to() {
    assert!(addressed_to_me(
        Some("dev"),
        None,
        None,
        None,
        Some("id-dev-1"),
        "dev"
    ));
    let to = recipients(&["dev"]);
    assert!(addressed_to_me(
        None,
        None,
        Some(&to),
        None,
        Some("id-dev-1"),
        "dev"
    ));
}

#[test]
fn addressee_stamped_before_any_dev_registers_matches_the_later_dev_by_display() {
    assert!(
        addressed_to_me(Some("dev"), None, None, None, Some("id-dev-1"), "dev"),
        "an @dev sent before ANY dev registered stamps addressee_ids None (ingress freezes only complete resolutions) — the dev who registers later matches by display fallback instead of being locked out by a frozen empty id set"
    );
    assert!(
        !addressed_to_me(Some("dev"), None, None, None, Some("id-pm-1"), "pm"),
        "the display fallback is still display-precise — a different display never matches"
    );
}

#[test]
fn addressed_to_me_is_un_muteable_even_when_class_off() {
    assert!(
        chat_class_reaches_live_stream("nudge", true, false, &classes(&["chat"])),
        "#1648/1a: addressed-to-me is the floor — pushes regardless of relevance-set or class"
    );
}

#[test]
fn owner_sender_is_un_muteable_even_with_empty_set() {
    assert!(
        chat_class_reaches_live_stream("nudge", false, true, &[]),
        "#1648/1a: owner-sent is the floor — un-muteable even under the curated default"
    );
}

#[test]
fn curated_default_pushes_broadcast_chat() {
    assert!(
        chat_class_reaches_live_stream("chat", false, false, &[]),
        "#1648/1a: empty set => curated default keeps broadcast-chat ON (team awareness)"
    );
}

#[test]
fn curated_default_pushes_insight() {
    assert!(
        chat_class_reaches_live_stream("insight", false, false, &[]),
        "#1648/1a: insight not in the OFF-set (activity/system/others-nudges) => default ON"
    );
}

#[test]
fn curated_default_suppresses_non_addressed_nudge() {
    assert!(
        !chat_class_reaches_live_stream("nudge", false, false, &[]),
        "#1648/1a: empty set => curated default suppresses others'-nudges from the live-push"
    );
}

#[test]
fn custom_set_pushes_in_set_class() {
    assert!(
        chat_class_reaches_live_stream("chat", false, false, &classes(&["chat", "insight"])),
        "#1648/1a: declared relevance-set => in-set class is pushed"
    );
}

#[test]
fn custom_set_suppresses_out_of_set_class_for_same_agent() {
    assert!(
        !chat_class_reaches_live_stream("insight", false, false, &classes(&["chat"])),
        "#1648/1a: SAME agent, declared chat-only => insight (out-of-set) suppressed — non-vacuous discrimination"
    );
}

#[test]
fn custom_set_suppresses_broadcast_chat_when_muted() {
    assert!(
        !chat_class_reaches_live_stream("chat", false, false, &classes(&["insight"])),
        "#1648/1a: an agent can MUTE broadcast-chat by declaring a set that omits it"
    );
}

#[test]
fn floor_wins_over_custom_set_omission() {
    assert!(
        chat_class_reaches_live_stream("chat", true, false, &classes(&["insight"])),
        "#1648/1a: addressed-to-me floor overrides a relevance-set that omits the class"
    );
}
