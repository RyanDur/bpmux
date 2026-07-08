#![allow(unused_imports)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use httpmock::{Method, MockServer};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use super::{
    board_author, board_issue, board_router, board_test_session, post_json, reaction_toggle,
    response_json, tracker_for,
};

use super::{init_session, seed_messages, seed_mixed_messages, test_app};
use crate::hub;
use crate::message::MessageType;
use crate::transport::rest::*;

#[test]
fn build_reaction_event_text_names_sender_from_message_key() {
    let body = reaction_toggle("owner", "👍", "2026-04-23T11:30:00Z|dev:claude-code");
    let text = build_reaction_event_text_with_excerpt(&body, true, None);
    assert!(
        text.contains("owner") && text.contains("👍") && text.contains("dev"),
        "text should name reactor + emoji + sender: {text}"
    );
    assert!(
        text.contains("reacted") && !text.contains("unreacted"),
        "present=true should use 'reacted' verb: {text}"
    );
}

#[test]
fn build_reaction_event_text_unreact_uses_unreacted_verb() {
    let body = reaction_toggle("dev", "🎉", "2026-04-23T11:30:00Z|pm:claude-code");
    let text = build_reaction_event_text_with_excerpt(&body, false, None);
    assert!(
        text.contains("unreacted"),
        "should use 'unreacted' verb: {text}"
    );
    let starts_with_reacted = text.starts_with("reacted ") || text.contains(" reacted ");
    assert!(
        !starts_with_reacted,
        "present=false must not include standalone 'reacted' word: {text}"
    );
    assert!(text.contains("dev") && text.contains("🎉") && text.contains("pm"));
}

#[test]
fn build_reaction_event_text_falls_back_without_message_key_shape() {
    let body = reaction_toggle("owner", "✨", "malformed-key");
    let text = build_reaction_event_text_with_excerpt(&body, true, None);
    assert!(text.contains("owner") && text.contains("✨"));
    assert!(
        !text.contains("'s message"),
        "no sender context when key is malformed: {text}"
    );
}

#[test]
fn build_reaction_event_text_strips_transport_suffix_from_sender() {
    let body = reaction_toggle("owner", "👍", "2026-04-23T11:30:00Z|designer:claude-code");
    let text = build_reaction_event_text_with_excerpt(&body, true, None);
    assert!(
        text.contains("designer") && !text.contains(":claude-code"),
        "sender should render as short-role: {text}"
    );
}
