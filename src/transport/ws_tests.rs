#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::ws::*;
use crate::session;
use chrono::Utc;

fn msg(from: &str, to: Option<Vec<&str>>) -> session::Message {
    session::Message {
        from: from.to_string(),
        text: "test".to_string(),
        ts: Utc::now(),
        message_type: None,
        to: to.map(|v| v.into_iter().map(String::from).collect()),
    }
}

#[test]
fn broadcast_delivered_to_everyone() {
    assert!(should_deliver(
        &msg("dev:claude-code", None),
        "pm:claude-code"
    ));
    assert!(should_deliver(&msg("dev:claude-code", None), "ryan:human"));
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
fn group_addressed_delivered_to_all_named() {
    let m = msg("dev:claude-code", Some(vec!["pm", "designer"]));
    assert!(should_deliver(&m, "pm:claude-code"));
    assert!(should_deliver(&m, "designer:claude-code"));
    assert!(should_deliver(&m, "dev:human"));
    assert!(!should_deliver(&m, "ryan:human"));
}

#[test]
fn to_all_is_broadcast() {
    let m = msg("dev:claude-code", Some(vec!["all"]));
    assert!(should_deliver(&m, "pm:claude-code"));
    assert!(should_deliver(&m, "ryan:human"));
    assert!(should_deliver(&m, "designer:claude-code"));
}

#[test]
fn short_name_strips_tool_suffix() {
    assert_eq!(short_name("dev:claude-code"), "dev");
    assert_eq!(short_name("ryan:human"), "ryan");
    assert_eq!(short_name("bpmux"), "bpmux");
}
