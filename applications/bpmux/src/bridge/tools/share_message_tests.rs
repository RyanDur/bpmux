#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::share_message::*;
use crate::transport::protocol::ClientMessage;

fn private_source() -> serde_json::Value {
    serde_json::json!({
        "from": "designer:claude-code",
        "text": "confidential design note",
        "explicit_privacy": true,
        "to": ["dev"]
    })
}

fn shareable_broadcast_source() -> serde_json::Value {
    serde_json::json!({
        "from": "pm:claude-code",
        "text": "open broadcast note",
        "to": serde_json::Value::Null
    })
}

#[test]
fn reshare_refuses_explicit_privacy_source_and_does_not_widen() {
    let err = build_reshare_message(&private_source(), "pm", Some(vec!["all".to_string()]))
        .expect_err("an explicit_privacy source must be refused");
    assert!(
        err.contains("cannot share a private message"),
        "refusal names the privacy cause; got {err}"
    );
    assert!(
        build_reshare_message(&private_source(), "pm", None).is_err(),
        "a private source must never produce a ClientMessage::Send — no widening to any scope"
    );
}

#[test]
fn reshare_drops_addressee_on_propagated_message() {
    match build_reshare_message(
        &shareable_broadcast_source(),
        "dev",
        Some(vec!["all".into()]),
    )
    .expect("non-private source builds a Send")
    {
        ClientMessage::Send { addressee, .. } => assert_eq!(
            addressee, None,
            "propagated message must not inherit the source addressee"
        ),
        other => panic!("expected ClientMessage::Send, got {other:?}"),
    }
}

#[test]
fn reshare_marks_source_scope_broadcast_when_source_has_no_recipients() {
    match build_reshare_message(&shareable_broadcast_source(), "dev", None).unwrap() {
        ClientMessage::Send { text, .. } => assert!(
            text.contains("from broadcast thread"),
            "no-recipient source marks broadcast provenance; got {text}"
        ),
        other => panic!("expected Send, got {other:?}"),
    }
}

#[test]
fn reshare_marks_source_scope_counterpart_for_dm() {
    let dm = serde_json::json!({"from": "designer:claude-code", "text": "x", "to": ["dev"]});
    match build_reshare_message(&dm, "dev", None).unwrap() {
        ClientMessage::Send { text, .. } => assert!(
            text.contains("from designer thread"),
            "DM source marks the counterpart as provenance scope; got {text}"
        ),
        other => panic!("expected Send, got {other:?}"),
    }
}

#[test]
fn format_share_marker_form1_includes_source_scope() {
    assert_eq!(
        format_share_marker("dev", Some("pm"), "body"),
        "[Shared by dev from pm thread] body"
    );
}

#[test]
fn format_share_marker_form2_omits_source_scope() {
    assert_eq!(
        format_share_marker("dev", None, "body"),
        "[Shared by dev] body"
    );
}

#[test]
fn parse_share_scope_all_widens_to_broadcast() {
    assert_eq!(parse_share_scope(message::BROADCAST_ALL), Ok(None));
}

#[test]
fn parse_share_scope_comma_list_splits_into_recipients() {
    assert_eq!(
        parse_share_scope("pm, designer"),
        Ok(Some(vec!["pm".to_string(), "designer".to_string()]))
    );
}

#[test]
fn parse_share_scope_empty_or_whitespace_is_err() {
    assert!(parse_share_scope("").is_err());
    assert!(parse_share_scope("  ,  ").is_err());
}

#[test]
fn pick_most_recent_skips_non_chat_then_returns_chat() {
    let messages = vec![
        serde_json::json!({"message_type": "activity", "text": "act"}),
        serde_json::json!({"message_type": serde_json::Value::Null, "text": "real chat"}),
    ];
    assert_eq!(
        pick_most_recent_visible_chat(&messages).unwrap()["text"],
        "real chat"
    );
}

#[test]
fn pick_most_recent_treats_missing_message_type_as_chat() {
    let messages = vec![serde_json::json!({"text": "untyped chat"})];
    assert!(pick_most_recent_visible_chat(&messages).is_some());
}

#[test]
fn pick_most_recent_none_when_all_non_chat() {
    let messages = vec![serde_json::json!({"message_type": "system", "text": "sys"})];
    assert!(pick_most_recent_visible_chat(&messages).is_none());
}
