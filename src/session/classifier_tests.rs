#![allow(clippy::unwrap_used)]

use super::classifier::{MessageClass, classify};
use super::message::{Message, MessageType};
use chrono::Utc;

fn msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
    }
}

#[test]
fn owner_question_is_correction() {
    let m = msg("ryan:human", "why did you do that?");
    assert_eq!(classify(&m, "ryan"), MessageClass::Correction);
}

#[test]
fn owner_directive_is_correction() {
    let m = msg("ryan:human", "don't test private functions");
    assert_eq!(classify(&m, "ryan"), MessageClass::Correction);
}

#[test]
fn owner_should_is_correction() {
    let m = msg("ryan:human", "you should restate before building");
    assert_eq!(classify(&m, "ryan"), MessageClass::Correction);
}

#[test]
fn owner_acceptance_is_decision() {
    let m = msg("ryan:human", "accepted");
    assert_eq!(classify(&m, "ryan"), MessageClass::Decision);
}

#[test]
fn owner_plain_chat_is_discussion() {
    let m = msg("ryan:human", "sounds good, keep going");
    assert_eq!(classify(&m, "ryan"), MessageClass::Discussion);
}

#[test]
fn agent_message_is_discussion() {
    let m = msg("dev:claude-code", "I'll fix the filter now");
    assert_eq!(classify(&m, "ryan"), MessageClass::Discussion);
}

#[test]
fn agent_story_decision_is_decision() {
    let m = msg("pm:claude-code", "#92 accepted and closed");
    assert_eq!(classify(&m, "ryan"), MessageClass::Decision);
}

#[test]
fn system_message_is_routine() {
    let m = Message {
        from: "bpmux".to_string(),
        text: "dev:ws joined".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
    };
    assert_eq!(classify(&m, "ryan"), MessageClass::Routine);
}

#[test]
fn short_owner_question_is_not_correction() {
    let m = msg("ryan:human", "pm, you in?");
    assert_eq!(classify(&m, "ryan"), MessageClass::Discussion);
}

#[test]
fn long_owner_question_is_correction() {
    let m = msg(
        "ryan:human",
        "is there a way for you to look at it before you show me?",
    );
    assert_eq!(classify(&m, "ryan"), MessageClass::Correction);
}
