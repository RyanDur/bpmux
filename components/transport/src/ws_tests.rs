#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::ws::*;
use chrono::Utc;
use protocol::{ClientMessage, ServerMessage};

#[test]
fn reaction_toggle_frame_deserializes_with_snake_case_fields() {
    let wire = r#"{"type":"reaction_toggle","channel":"broadcast","message_key":"2026-04-22T18:30:00Z|dev:claude-code","emoji":"👍","user":"owner"}"#;
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
        assert_eq!(user, "owner");
    }
}

#[test]
fn comments_hydrate_frame_deserializes_with_snake_case_story_id() {
    let wire = r#"{"type":"comments_hydrate","story_id":366}"#;
    let parsed: ClientMessage =
        serde_json::from_str(wire).expect("renderer comments_hydrate frame must deserialize");
    assert!(
        matches!(&parsed, ClientMessage::CommentsHydrate { story_id: 366 }),
        "expected CommentsHydrate{{366}}, got {parsed:?}"
    );
}

#[test]
fn comments_hydrated_reply_serializes_to_the_caller_only_frame() {
    let msg = ServerMessage::CommentsHydrated {
        story_id: 366,
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
        json.contains(r#""story_id":366"#),
        "missing story_id: {json}"
    );
    assert!(
        json.contains(r#""author":"designer""#),
        "missing comment payload: {json}"
    );
}

#[test]
fn timeline_hydrate_frame_deserializes_with_snake_case_story_id() {
    let wire = r#"{"type":"timeline_hydrate","story_id":366}"#;
    let parsed: ClientMessage =
        serde_json::from_str(wire).expect("renderer timeline_hydrate frame must deserialize");
    assert!(
        matches!(&parsed, ClientMessage::TimelineHydrate { story_id: 366 }),
        "expected TimelineHydrate{{366}}, got {parsed:?}"
    );
}

#[test]
fn timeline_hydrated_reply_serializes_to_the_caller_only_frame() {
    let msg = ServerMessage::TimelineHydrated {
        story_id: 366,
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
        json.contains(r#""story_id":366"#),
        "missing story_id: {json}"
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

#[test]
fn message_size_ok_returns_true_for_short_ping_sized_text() {
    let typical = "pm: 🖐 pm — distinct angle on the sequencing";
    assert!(message_size_ok(typical));
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
    assert_eq!(msg.message_type, message::MessageType::System);
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
