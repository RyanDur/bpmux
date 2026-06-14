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

fn nudge(to: Option<Vec<&str>>) -> session::Message {
    session::Message {
        from: "bpmux".to_string(),
        text: "📝 candidate moment — save_lesson".to_string(),
        ts: Utc::now(),
        message_type: session::MessageType::Nudge,
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
fn full_identity_recipient_delivers_to_matching_peer() {
    let m = msg("dev:claude-code", Some(vec!["pm:claude-code"]));
    assert!(
        should_deliver(&m, "pm:claude-code"),
        "#1568: a full-identity `to:` recipient must match its peer, like sender/peer are already normalized"
    );
    assert!(
        !should_deliver(&m, "designer:claude-code"),
        "#1568: recipient-form normalization must not over-deliver to a non-recipient"
    );
}

#[test]
fn group_addressed_delivered_to_all_named() {
    let m = msg("dev:claude-code", Some(vec!["pm", "designer"]));
    assert!(should_deliver(&m, "pm:claude-code"));
    assert!(should_deliver(&m, "designer:claude-code"));
    assert!(should_deliver(&m, "dev:human"));
    assert!(
        should_deliver(&m, "ryan:human"),
        "owner-omniscience: the owner receives a group conversation even though it is NOT named in --to — scope spares peers, never hides from the owner"
    );
}

#[test]
fn owner_receives_every_conversation_even_scoped_to_other_peers() {
    let m = msg("dev:claude-code", Some(vec!["pm", "designer"]));
    assert!(
        should_deliver(&m, "ryan:human"),
        "owner-omniscience (owner-raised): a chat message scoped to:[pm,designer] STILL reaches the owner — --to is peer-attention, never owner-hiding. The owner sees every conversation; the all-conversations view renders it while group rooms still filter by --to."
    );
    assert!(
        !should_deliver(&m, "designer-x:claude-code"),
        "owner-omniscience does NOT loosen peer scoping — a non-owner not in --to is still excluded"
    );
}

#[test]
fn owner_receives_activity_but_nudges_stay_substrate_excluded() {
    let mut activity = msg("dev:claude-code", None);
    activity.message_type = session::MessageType::Activity;
    assert!(
        should_deliver(&activity, "ryan:human"),
        "the owner's SESSION receives activity signals (they were already broadcast) — hiding plumbing is the RENDERER's job (events-class hidden from all-conversations by default, FilterPills-toggleable), not the substrate's"
    );
    assert!(
        !should_deliver(&nudge(Some(vec!["ryan"])), "ryan:human"),
        "owner-omniscience PRESERVES the deliberate #1525 substrate nudge-exclusion — nudges (recall/handoff routing) never reach the owner-peer; surfacing them is a separate #1525-revisit, not this fix"
    );
}

#[test]
fn to_all_is_broadcast() {
    let m = msg("dev:claude-code", Some(vec!["all"]));
    assert!(should_deliver(&m, "pm:claude-code"));
    assert!(should_deliver(&m, "ryan:human"));
    assert!(should_deliver(&m, "designer:claude-code"));
}

#[test]
fn nudge_never_delivered_to_owner_even_when_addressed_to_owner() {
    assert!(
        !should_deliver(&nudge(Some(vec!["ryan"])), "ryan:human"),
        "#1525: a Nudge addressed to the owner must NEVER reach the owner-role peer (recall/handoff can address it to: owner)"
    );
}

#[test]
fn nudge_never_delivered_to_owner_on_broadcast_or_all_paths() {
    assert!(
        !should_deliver(&nudge(None), "ryan:human"),
        "#1525: a broadcast Nudge (to:None) must not slip through to the owner"
    );
    assert!(
        !should_deliver(&nudge(Some(vec!["all"])), "ryan:human"),
        "#1525: a to:all Nudge must not slip through to the owner"
    );
}

#[test]
fn nudge_still_delivered_to_every_agent_role() {
    assert!(
        should_deliver(&nudge(Some(vec!["dev"])), "dev:claude-code"),
        "#1525: dev must still receive Nudges (valid agent-internal coordination)"
    );
    assert!(
        should_deliver(&nudge(Some(vec!["pm"])), "pm:claude-code"),
        "#1525: pm must still receive Nudges"
    );
    assert!(
        should_deliver(&nudge(Some(vec!["designer"])), "designer:claude-code"),
        "#1525: designer must still receive Nudges"
    );
    assert!(
        should_deliver(&nudge(Some(vec!["all"])), "dev:claude-code"),
        "#1525: a to:all Nudge must still reach agents (only the owner is excluded)"
    );
}

#[test]
fn short_name_strips_tool_suffix() {
    assert_eq!(short_name("dev:claude-code"), "dev");
    assert_eq!(short_name("ryan:human"), "ryan");
    assert_eq!(short_name("bpmux"), "bpmux");
}

#[test]
fn message_size_ok_returns_true_for_short_ping_sized_text() {
    let typical = "pm: 🖐 pm — distinct angle on the sequencing";
    assert!(message_size_ok(typical));
}

#[test]
fn message_size_ok_returns_true_for_3kb_text() {
    let text = "x".repeat(3 * 1024);
    assert!(message_size_ok(&text));
}

#[test]
fn message_size_ok_returns_true_for_5kb_text() {
    let text = "x".repeat(5 * 1024);
    assert!(message_size_ok(&text));
}

#[test]
fn message_size_ok_returns_true_for_32kb_text() {
    let text = "x".repeat(32 * 1024);
    assert!(message_size_ok(&text));
}

#[test]
fn message_size_ok_returns_false_for_1mb_text() {
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
