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
fn owner_why_question_is_correction() {
    let m = msg(
        "ryan:human",
        "why did you do that instead of the simpler approach?",
    );
    assert_eq!(classify(&m, "ryan"), MessageClass::Correction);
}

#[test]
fn owner_you_should_is_correction() {
    let m = msg("ryan:human", "you should restate before building");
    assert_eq!(classify(&m, "ryan"), MessageClass::Correction);
}

#[test]
fn owner_go_through_is_correction() {
    let m = msg(
        "ryan:human",
        "design feedback should go through the designer",
    );
    assert_eq!(classify(&m, "ryan"), MessageClass::Correction);
}

#[test]
fn owner_quality_is_correction() {
    let m = msg("ryan:human", "quality is the goal. in everything");
    assert_eq!(classify(&m, "ryan"), MessageClass::Correction);
}

#[test]
fn owner_focus_on_is_correction() {
    let m = msg("ryan:human", "focus on the problem, not the fix");
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
fn owner_restart_question_is_discussion() {
    let m = msg(
        "ryan:human",
        "do I need to restart everything or just bpmux?",
    );
    assert_eq!(classify(&m, "ryan"), MessageClass::Discussion);
}

#[test]
fn owner_screenshot_is_discussion() {
    let m = msg(
        "ryan:human",
        "/Users/ryandurling/IdeaProjects/bpmux/screen-shots/img_5.png",
    );
    assert_eq!(classify(&m, "ryan"), MessageClass::Discussion);
}

#[test]
fn owner_keep_going_is_discussion() {
    let m = msg("ryan:human", "great, keep going");
    assert_eq!(classify(&m, "ryan"), MessageClass::Discussion);
}

#[test]
fn owner_yes_is_discussion() {
    let m = msg("ryan:human", "yes");
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
