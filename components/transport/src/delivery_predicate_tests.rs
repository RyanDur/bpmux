#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::ws::*;
use chrono::Utc;

fn msg(from: &str, to: Option<Vec<&str>>) -> message::Message {
    message::Message {
        from: from.to_string(),
        text: "test".to_string(),
        ts: Utc::now(),
        to: to.map(|v| v.into_iter().map(String::from).collect()),
        ..message::Message::default()
    }
}

fn nudge(to: Option<Vec<&str>>) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: "📝 candidate moment — save_lesson".to_string(),
        ts: Utc::now(),
        message_type: message::MessageType::Nudge,
        to: to.map(|v| v.into_iter().map(String::from).collect()),
        ..message::Message::default()
    }
}

const AGENT: bool = false;
const OWNER: bool = true;

#[test]
fn broadcast_delivered_to_everyone() {
    assert!(should_deliver(
        &msg("dev:claude-code", None),
        None,
        "pm:claude-code",
        AGENT,
    ));
    assert!(should_deliver(
        &msg("dev:claude-code", None),
        None,
        "owner:human",
        OWNER,
    ));
}

#[test]
fn addressed_delivered_to_recipient() {
    let m = msg("dev:claude-code", Some(vec!["pm"]));
    assert!(should_deliver(&m, None, "pm:claude-code", AGENT));
}

#[test]
fn addressed_delivered_to_sender() {
    let m = msg("dev:claude-code", Some(vec!["pm"]));
    assert!(should_deliver(&m, None, "dev:claude-code", AGENT));
}

#[test]
fn addressed_not_delivered_to_non_recipient() {
    let m = msg("dev:claude-code", Some(vec!["pm"]));
    assert!(!should_deliver(&m, None, "designer:claude-code", AGENT));
}

#[test]
fn full_identity_recipient_delivers_to_matching_peer() {
    let m = msg("dev:claude-code", Some(vec!["pm:claude-code"]));
    assert!(
        should_deliver(&m, None, "pm:claude-code", AGENT),
        "#1568: a full-identity `to:` recipient must match its peer, like sender/peer are already normalized"
    );
    assert!(
        !should_deliver(&m, None, "designer:claude-code", AGENT),
        "#1568: recipient-form normalization must not over-deliver to a non-recipient"
    );
}

#[test]
fn group_addressed_delivered_to_all_named() {
    let m = msg("dev:claude-code", Some(vec!["pm", "designer"]));
    assert!(should_deliver(&m, None, "pm:claude-code", AGENT));
    assert!(should_deliver(&m, None, "designer:claude-code", AGENT));
    assert!(should_deliver(&m, None, "dev:human", AGENT));
    assert!(
        should_deliver(&m, None, "owner:human", OWNER),
        "owner-omniscience: the owner receives a group conversation even though it is NOT named in --to — scope spares peers, never hides from the owner"
    );
}

#[test]
fn owner_mark_receives_every_conversation_scoped_to_other_peers_regardless_of_name() {
    let m = msg("dev:claude-code", Some(vec!["pm", "designer"]));
    assert!(
        should_deliver(&m, None, "owner:human", OWNER),
        "owner-omniscience: the owner MARK (not the display string) receives a chat scoped to:[pm,designer] — --to is peer-attention, never owner-hiding"
    );
    assert!(
        !should_deliver(&m, None, "owner:electron", AGENT),
        "THE centerpiece at the delivery seam: a session presenting the display 'owner' but WITHOUT the owner mark is scoped out exactly like any non-recipient — self-declared owner-ness grants nothing"
    );
    assert!(
        !should_deliver(&m, None, "designer-x:claude-code", AGENT),
        "owner-omniscience does NOT loosen peer scoping — a non-owner not in --to is still excluded"
    );
}

#[test]
fn owner_mark_receives_activity_but_nudges_stay_substrate_excluded() {
    let mut activity = msg("dev:claude-code", None);
    activity.message_type = message::MessageType::Activity;
    assert!(
        should_deliver(&activity, None, "owner:human", OWNER),
        "the owner's SESSION receives activity signals (they were already broadcast) — hiding plumbing is the RENDERER's job (events-class hidden from all-conversations by default, FilterPills-toggleable), not the substrate's"
    );
    assert!(
        !should_deliver(&nudge(Some(vec!["owner"])), None, "owner:human", OWNER),
        "owner-omniscience PRESERVES the deliberate #1525 substrate nudge-exclusion — nudges (recall/handoff routing) never reach the owner-peer; surfacing them is a separate #1525-revisit, not this fix"
    );
}

#[test]
fn to_all_is_broadcast() {
    let m = msg("dev:claude-code", Some(vec!["all"]));
    assert!(should_deliver(&m, None, "pm:claude-code", AGENT));
    assert!(should_deliver(&m, None, "owner:human", OWNER));
    assert!(should_deliver(&m, None, "designer:claude-code", AGENT));
}

#[test]
fn nudge_never_delivered_to_owner_even_when_addressed_to_owner() {
    assert!(
        !should_deliver(&nudge(Some(vec!["owner"])), None, "owner:human", OWNER),
        "#1525: a Nudge addressed to the owner must NEVER reach the owner-role peer (recall/handoff can address it to: owner)"
    );
}

#[test]
fn nudge_never_delivered_to_owner_on_broadcast_or_all_paths() {
    assert!(
        !should_deliver(&nudge(None), None, "owner:human", OWNER),
        "#1525: a broadcast Nudge (to:None) must not slip through to the owner"
    );
    assert!(
        !should_deliver(&nudge(Some(vec!["all"])), None, "owner:human", OWNER),
        "#1525: a to:all Nudge must not slip through to the owner"
    );
}

#[test]
fn nudge_still_delivered_to_every_agent_role() {
    assert!(
        should_deliver(&nudge(Some(vec!["dev"])), None, "dev:claude-code", AGENT),
        "#1525: dev must still receive Nudges (valid agent-internal coordination)"
    );
    assert!(
        should_deliver(&nudge(Some(vec!["pm"])), None, "pm:claude-code", AGENT),
        "#1525: pm must still receive Nudges"
    );
    assert!(
        should_deliver(
            &nudge(Some(vec!["designer"])),
            None,
            "designer:claude-code",
            AGENT
        ),
        "#1525: designer must still receive Nudges"
    );
    assert!(
        should_deliver(&nudge(Some(vec!["all"])), None, "dev:claude-code", AGENT),
        "#1525: a to:all Nudge must still reach agents (only the owner is excluded)"
    );
}

#[test]
fn to_ids_precision_excludes_a_display_sibling_session() {
    let mut m = msg("pm:claude-code", Some(vec!["dev"]));
    m.to_ids = Some(vec!["id-dev-1".to_string()]);
    assert!(
        should_deliver(&m, Some("id-dev-1"), "dev:claude-code", AGENT),
        "the session identified as id-dev-1 is the frozen to_ids target"
    );
    assert!(
        !should_deliver(&m, Some("id-dev-2"), "dev:claude-code", AGENT),
        "id-dev-2 shares the display 'dev' with id-dev-1 but is a DIFFERENT user — id precision must not deliver to it just because the display matches"
    );
}

#[test]
fn legacy_row_with_no_frozen_to_ids_reaches_every_id_sharing_the_display() {
    let m = msg("pm:claude-code", Some(vec!["dev"]));
    assert!(m.to_ids.is_none(), "a legacy row has no frozen ids");
    assert!(should_deliver(
        &m,
        Some("id-dev-1"),
        "dev:claude-code",
        AGENT
    ));
    assert!(should_deliver(
        &m,
        Some("id-dev-2"),
        "dev:claude-code",
        AGENT
    ));
}

#[test]
fn sender_sees_own_message_by_id_even_when_to_ids_excludes_it() {
    let mut m = msg("dev:claude-code", Some(vec!["pm"]));
    m.sender_id = Some("id-dev-1".to_string());
    m.to_ids = Some(vec!["id-pm-1".to_string()]);
    assert!(
        should_deliver(&m, Some("id-dev-1"), "dev:claude-code", AGENT),
        "the sender always sees its own message even though the frozen to_ids excludes it"
    );
}
