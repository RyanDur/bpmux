#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::ws::*;
use crate::session;
use crate::transport::protocol::{ClientMessage, ServerMessage};
use chrono::Utc;

#[test]
fn reaction_toggle_frame_deserializes_with_snake_case_fields() {
    let wire = r#"{"type":"reaction_toggle","channel":"broadcast","message_key":"2026-04-22T18:30:00Z|dev:claude-code","emoji":"👍","user":"ryan"}"#;
    let parsed: ClientMessage =
        serde_json::from_str(wire).expect("renderer reaction_toggle frame must deserialize");
    assert!(
        matches!(&parsed, ClientMessage::ReactionToggle { .. }),
        "expected ReactionToggle, got {parsed:?}"
    );
    if let ClientMessage::ReactionToggle {
        channel,
        message_key,
        emoji,
        user,
    } = parsed
    {
        assert_eq!(channel, "broadcast");
        assert_eq!(message_key, "2026-04-22T18:30:00Z|dev:claude-code");
        assert_eq!(emoji, "👍");
        assert_eq!(user, "ryan");
    }
}

#[test]
fn comments_hydrate_frame_deserializes_with_snake_case_story_number() {
    let wire = r#"{"type":"comments_hydrate","story_number":366}"#;
    let parsed: ClientMessage =
        serde_json::from_str(wire).expect("renderer comments_hydrate frame must deserialize");
    assert!(
        matches!(
            &parsed,
            ClientMessage::CommentsHydrate { story_number: 366 }
        ),
        "expected CommentsHydrate{{366}}, got {parsed:?}"
    );
}

#[test]
fn comments_hydrated_reply_serializes_to_the_caller_only_frame() {
    let msg = ServerMessage::CommentsHydrated {
        story_number: 366,
        comments: vec![serde_json::json!({
            "id": 7700,
            "author": "designer",
            "body": "selection-based quoting",
            "created_at": "2026-04-22T15:20:00Z"
        })],
    };
    let json = serde_json::to_string(&msg).expect("serialize CommentsHydrated");
    assert!(
        json.contains(r#""type":"comments_hydrated""#),
        "missing type tag: {json}"
    );
    assert!(
        json.contains(r#""story_number":366"#),
        "missing story_number: {json}"
    );
    assert!(
        json.contains(r#""author":"designer""#),
        "missing comment payload: {json}"
    );
}

#[test]
fn timeline_hydrate_frame_deserializes_with_snake_case_story_number() {
    let wire = r#"{"type":"timeline_hydrate","story_number":366}"#;
    let parsed: ClientMessage =
        serde_json::from_str(wire).expect("renderer timeline_hydrate frame must deserialize");
    assert!(
        matches!(
            &parsed,
            ClientMessage::TimelineHydrate { story_number: 366 }
        ),
        "expected TimelineHydrate{{366}}, got {parsed:?}"
    );
}

#[test]
fn timeline_hydrated_reply_serializes_to_the_caller_only_frame() {
    let msg = ServerMessage::TimelineHydrated {
        story_number: 366,
        events: vec![serde_json::json!({
            "event": "labeled",
            "actor": "dev",
            "ts": "2026-04-22T15:20:00Z",
            "label": "delivered"
        })],
    };
    let json = serde_json::to_string(&msg).expect("serialize TimelineHydrated");
    assert!(
        json.contains(r#""type":"timeline_hydrated""#),
        "missing type tag: {json}"
    );
    assert!(
        json.contains(r#""story_number":366"#),
        "missing story_number: {json}"
    );
    assert!(
        json.contains(r#""label":"delivered""#),
        "missing timeline event payload: {json}"
    );
}

#[test]
fn history_hydrate_frame_deserializes_with_snake_case_limits() {
    let wire = r#"{"type":"history_hydrate","chat_limit":200,"activity_limit":50}"#;
    let parsed: ClientMessage =
        serde_json::from_str(wire).expect("renderer history_hydrate frame must deserialize");
    assert!(
        matches!(
            &parsed,
            ClientMessage::HistoryHydrate {
                chat_limit: 200,
                activity_limit: 50
            }
        ),
        "expected HistoryHydrate{{200,50}}, got {parsed:?}"
    );
}

#[test]
fn history_hydrated_reply_serializes_to_the_caller_only_frame() {
    let msg = ServerMessage::HistoryHydrated {
        messages: vec![serde_json::json!({
            "from": "dev",
            "text": "hello",
            "ts": "2026-04-22T15:20:00Z"
        })],
    };
    let json = serde_json::to_string(&msg).expect("serialize HistoryHydrated");
    assert!(
        json.contains(r#""type":"history_hydrated""#),
        "missing type tag: {json}"
    );
    assert!(
        json.contains(r#""text":"hello""#),
        "missing history message payload: {json}"
    );
}

#[test]
fn history_page_frame_deserializes_with_request_id_and_cursor() {
    let wire =
        r#"{"type":"history_page","request_id":7,"before":"2026-05-03T03:00:00Z","limit":200}"#;
    let parsed: ClientMessage =
        serde_json::from_str(wire).expect("renderer history_page frame must deserialize");
    assert!(
        matches!(
            &parsed,
            ClientMessage::HistoryPage {
                request_id: 7,
                limit: 200,
                ..
            }
        ),
        "expected HistoryPage{{7,...,200}}, got {parsed:?}"
    );
}

#[test]
fn history_page_loaded_reply_echoes_request_id_caller_only() {
    let msg = ServerMessage::HistoryPageLoaded {
        request_id: 7,
        messages: vec![serde_json::json!({
            "from": "pm",
            "text": "older",
            "ts": "2026-05-03T03:00:00Z"
        })],
    };
    let json = serde_json::to_string(&msg).expect("serialize HistoryPageLoaded");
    assert!(
        json.contains(r#""type":"history_page_loaded""#),
        "missing type tag: {json}"
    );
    assert!(
        json.contains(r#""request_id":7"#),
        "missing request_id echo: {json}"
    );
    assert!(
        json.contains(r#""text":"older""#),
        "missing page payload: {json}"
    );
}

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
