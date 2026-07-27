#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::*;
use message::Message;

#[test]
fn deserializes_identify_without_tool() {
    let json = r#"{"type": "identify", "name": "claude"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Identify {
            name,
            tool,
            cursor,
            user_id,
            owner_nonce,
        } => {
            assert_eq!(name, "claude");
            assert_eq!(tool, None);
            assert_eq!(cursor, None);
            assert_eq!(
                user_id, None,
                "an old client that omits user_id parses with an absent id — the server falls back to name-resolution (back-compat pin)"
            );
            assert_eq!(
                owner_nonce, None,
                "an old client that omits owner_nonce parses with an absent nonce — the additive field defaults, and the bootstrap fallback keeps it working (back-compat pin)"
            );
        }
        other => panic!("expected Identify, got {other:?}"),
    }
}

#[test]
fn deserializes_identify_with_owner_nonce() {
    let json = r#"{"type": "identify", "name": "owner", "tool": "electron", "owner_nonce": "launch-abc123"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Identify {
            name, owner_nonce, ..
        } => {
            assert_eq!(name, "owner");
            assert_eq!(
                owner_nonce,
                Some("launch-abc123".to_string()),
                "the launcher's per-launch nonce rides the identify frame — the backend marks THIS session the owner when it matches the claimed nonce"
            );
        }
        other => panic!("expected Identify, got {other:?}"),
    }
}

#[test]
fn deserializes_identify_with_tool() {
    let json = r#"{"type": "identify", "name": "claude", "tool": "claude-code"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Identify {
            name, tool, cursor, ..
        } => {
            assert_eq!(name, "claude");
            assert_eq!(tool, Some("claude-code".to_string()));
            assert_eq!(cursor, None);
        }
        other => panic!("expected Identify, got {other:?}"),
    }
}

#[test]
fn deserializes_identify_with_cursor() {
    let json = r#"{"type": "identify", "name": "claude", "cursor": "2026-06-04T06:00:00+00:00"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Identify { name, cursor, .. } => {
            assert_eq!(name, "claude");
            assert_eq!(cursor, Some("2026-06-04T06:00:00+00:00".to_string()));
        }
        other => panic!("expected Identify, got {other:?}"),
    }
}

#[test]
fn deserializes_identify_with_user_id() {
    let json =
        r#"{"type": "identify", "name": "dev", "tool": "claude-code", "user_id": "sess-dev-7f3a"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Identify { name, user_id, .. } => {
            assert_eq!(name, "dev");
            assert_eq!(
                user_id,
                Some("sess-dev-7f3a".to_string()),
                "a directory-held id arrives on the identify frame and becomes the roster key unconditionally (edge-registration pin)"
            );
        }
        other => panic!("expected Identify, got {other:?}"),
    }
}

#[test]
fn deserializes_send() {
    let json = r#"{"type": "send", "text": "hello"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Send { text, to, .. } => {
            assert_eq!(text, "hello");
            assert_eq!(to, None);
        }
        other => panic!("expected Send, got {other:?}"),
    }
}

#[test]
fn deserializes_pty_subscribe() {
    let json = r#"{"type": "pty_subscribe", "terminal_id": "term-1"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::PtySubscribe { terminal_id } => assert_eq!(terminal_id, "term-1"),
        other => panic!("expected PtySubscribe, got {other:?}"),
    }
}

#[test]
fn deserializes_pty_input_with_terminal_id_and_data() {
    let json = r#"{"type": "pty_input", "terminal_id": "term-1", "data": "Aw=="}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::PtyInput { terminal_id, data } => {
            assert_eq!(terminal_id, "term-1");
            assert_eq!(data, "Aw==");
        }
        other => panic!("expected PtyInput, got {other:?}"),
    }
}

#[test]
fn pty_output_serializes_with_distinct_type_terminal_id_and_data() {
    let msg = ServerMessage::PtyOutput {
        terminal_id: "term-1".to_string(),
        data: "G1szMW1yZWQ=".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(
        json.contains(r#""type":"pty_output""#),
        "distinct frame type: {json}"
    );
    assert!(
        json.contains(r#""terminal_id":"term-1""#),
        "carries terminal_id: {json}"
    );
    assert!(
        json.contains(r#""data":"G1szMW1yZWQ=""#),
        "carries data: {json}"
    );
}

#[test]
fn deserializes_pty_resize_with_terminal_id_cols_and_rows() {
    let json = r#"{"type": "pty_resize", "terminal_id": "term-1", "cols": 120, "rows": 40}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::PtyResize {
            terminal_id,
            cols,
            rows,
        } => {
            assert_eq!(terminal_id, "term-1");
            assert_eq!(cols, 120);
            assert_eq!(rows, 40);
        }
        other => panic!("expected PtyResize, got {other:?}"),
    }
}

#[test]
fn pty_exit_serializes_with_distinct_type_and_terminal_id() {
    let msg = ServerMessage::PtyExit {
        terminal_id: "term-1".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(
        json.contains(r#""type":"pty_exit""#),
        "distinct frame type: {json}"
    );
    assert!(
        json.contains(r#""terminal_id":"term-1""#),
        "carries terminal_id: {json}"
    );
}

#[test]
fn pty_mode_serializes_the_bracketed_paste_fact_per_terminal() {
    let msg = ServerMessage::PtyMode {
        terminal_id: "term-1".to_string(),
        bracketed_paste: true,
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(
        json.contains(r#""type":"pty_mode""#),
        "distinct frame type the renderer branches input-ownership on: {json}"
    );
    assert!(
        json.contains(r#""terminal_id":"term-1""#),
        "scoped to the terminal whose mode changed: {json}"
    );
    assert!(
        json.contains(r#""bracketed_paste":true"#),
        "carries the bracketed-paste fact (the raw-vs-cooked proxy) the agent's app set: {json}"
    );
}

#[test]
fn deserializes_send_with_to() {
    let json = r#"{"type": "send", "text": "hey", "to": ["pm", "designer"]}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Send { text, to, .. } => {
            assert_eq!(text, "hey");
            assert_eq!(to, Some(vec!["pm".to_string(), "designer".to_string()]));
        }
        other => panic!("expected Send, got {other:?}"),
    }
}

#[test]
fn serializes_welcome() {
    let msg = ServerMessage::Welcome {};
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(json.contains(r#""type":"welcome"#));
    assert!(
        !json.contains("session_id"),
        "welcome carries no session_id: {json}"
    );
}

#[test]
fn deserializes_rename() {
    let json = r#"{"type": "rename", "name": "new-name"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Rename { name } => assert_eq!(name, "new-name"),
        other => panic!("expected Rename, got {other:?}"),
    }
}

#[test]
fn serializes_message_from_session_message() {
    let session_msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: chrono::Utc::now(),
        ..Message::default()
    };
    let server_msg: ServerMessage = session_msg.into();
    let json = serde_json::to_string(&server_msg).expect("serialize");
    assert!(json.contains(r#""type":"message"#));
    assert!(json.contains(r#""from":"dev:human"#));
}

#[test]
fn message_frame_carries_the_delivery_id_fields_through_the_from_conversion() {
    let session_msg = Message {
        from: "dev:claude-code".to_string(),
        sender_id: Some("id-dev".to_string()),
        text: "hello".to_string(),
        ts: chrono::Utc::now(),
        to: Some(vec!["pm".to_string()]),
        to_ids: Some(vec!["id-pm".to_string()]),
        addressee: Some("pm".to_string()),
        addressee_ids: Some(vec!["id-pm".to_string()]),
        ..Message::default()
    };
    let server_msg: ServerMessage = session_msg.into();
    let json = serde_json::to_string(&server_msg).expect("serialize");
    assert!(
        json.contains(r#""sender_id":"id-dev""#),
        "the live ws chat frame must carry sender_id — the explicit-copy conversion drops it otherwise: {json}"
    );
    assert!(
        json.contains(r#""to_ids":["id-pm"]"#),
        "the live ws chat frame must carry to_ids: {json}"
    );
    assert!(
        json.contains(r#""addressee_ids":["id-pm"]"#),
        "the live ws chat frame must carry addressee_ids: {json}"
    );
}

#[test]
fn message_frame_without_delivery_id_fields_still_deserializes_bridge_side() {
    let json = r#"{"type":"message","from":"pm:claude-code","text":"legacy frame","ts":"2026-06-01T12:00:00Z"}"#;
    let msg: ServerMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ServerMessage::Message {
            from,
            sender_id,
            to_ids,
            addressee_ids,
            ..
        } => {
            assert_eq!(from, "pm:claude-code");
            assert_eq!(sender_id, None);
            assert_eq!(to_ids, None);
            assert_eq!(addressee_ids, None);
        }
        other => panic!("expected Message, got {other:?}"),
    }
}

#[test]
fn a_send_frame_with_only_known_fields_parses() {
    let json = r#"{"type": "send", "text": "hi", "to_ids": ["id-1"]}"#;
    let parsed = parse_client_frame(json).expect("known fields parse");
    assert!(matches!(parsed, ClientMessage::Send { .. }));
}

#[test]
fn a_send_frame_with_an_unknown_field_is_rejected_loudly_not_silently_broadcast() {
    let json = r#"{"type": "send", "text": "private thing", "to_groups": ["future-feature"]}"#;
    let error = parse_client_frame(json).expect_err(
        "an unknown routing field means version skew — the send must FAIL, never silently widen to broadcast (the to_ids rollout incident)",
    );
    assert!(
        error.contains("to_groups"),
        "the rejection names the unknown field; got: {error}"
    );
}

#[test]
fn non_send_frames_keep_their_tolerant_parse() {
    let json = r#"{"type": "identify", "name": "claude", "some_future_field": true}"#;
    assert!(
        parse_client_frame(json).is_ok(),
        "identify stays tolerant — evolving optional fields must not break the connect handshake"
    );
}

#[test]
fn the_send_field_allowlist_covers_every_field_the_struct_can_serialize() {
    let full = ClientMessage::Send {
        text: "t".to_string(),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
        message_type: Some("chat".to_string()),
        to_ids: Some(vec!["id-1".to_string()]),
    };
    let value: serde_json::Value = serde_json::to_value(&full).expect("send serializes");
    for key in value.as_object().expect("object").keys() {
        assert!(
            parse_client_frame(&value.to_string()).is_ok(),
            "a frame the CURRENT struct serializes must always parse strictly"
        );
        assert!(
            SEND_FIELDS.contains(&key.as_str()),
            "field '{key}' is serializable but missing from SEND_FIELDS — the allowlist rotted"
        );
    }
}

#[test]
fn the_server_chat_golden_is_exactly_what_the_rust_serializer_produces() {
    let canonical = ServerMessage::Message {
        from: "dev:claude-code".to_string(),
        sender_id: Some("6b3054e9-a6b4-4e5b-84db-f788ebdec25b".to_string()),
        text: "the canonical fully-routed chat frame".to_string(),
        ts: "2026-07-23T12:00:00Z".to_string(),
        to: Some(vec!["pm".to_string()]),
        to_ids: Some(vec!["pm-alpha".to_string()]),
        addressee: Some("pm".to_string()),
        addressee_ids: Some(vec!["pm-alpha".to_string()]),
        message_type: Some("chat".to_string()),
        multi_depth: Box::new(None),
        state_delta: None,
    };
    let golden: serde_json::Value =
        serde_json::from_str(include_str!("../golden/server-chat-message.json"))
            .expect("golden parses");
    assert_eq!(
        serde_json::to_value(&canonical).expect("serializes"),
        golden,
        "the golden IS the wire — if the Rust side changes shape, this fails HERE instead of at runtime in the renderer (the renderer decodes the same file in transport/golden.test.ts)"
    );
}

#[test]
fn the_client_send_golden_parses_strictly_and_serializes_back_to_itself() {
    let golden_text = include_str!("../golden/client-send.json");
    let parsed = parse_client_frame(golden_text).expect("the golden send parses strictly");
    let golden: serde_json::Value = serde_json::from_str(golden_text).expect("golden parses");
    assert_eq!(
        serde_json::to_value(&parsed).expect("serializes"),
        golden,
        "a send round-trips through the struct without gaining or losing fields"
    );
}
