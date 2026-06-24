#![allow(clippy::unwrap_used)]

use super::classifier::{ExperienceKind, MIN_EXPERIENCE_LEN, MessageClass, classify};
use crate::session::message::{Message, MessageType};
use chrono::Utc;

fn msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        ..Message::default()
    }
}

#[test]
fn owner_why_question_is_experience() {
    let m = msg(
        "owner:human",
        "why did you do that instead of the simpler approach?",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Question)
    );
}

#[test]
fn owner_you_should_is_teaching() {
    let m = msg("owner:human", "you should restate before building");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Teaching)
    );
}

#[test]
fn owner_go_through_is_teaching() {
    let m = msg(
        "owner:human",
        "design feedback should go through the designer",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Teaching)
    );
}

#[test]
fn owner_quality_is_teaching() {
    let m = msg("owner:human", "quality is the goal. in everything");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Teaching)
    );
}

#[test]
fn owner_focus_on_is_teaching() {
    let m = msg("owner:human", "focus on the problem, not the fix");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Teaching)
    );
}

#[test]
fn owner_dont_do_is_correction() {
    let m = msg("owner:human", "don't do that, it breaks the build");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn owner_acceptance_is_decision() {
    let m = msg("owner:human", "accepted");
    assert_eq!(classify(&m, "owner"), MessageClass::Decision);
}

#[test]
fn owner_plain_chat_is_discussion() {
    let m = msg("owner:human", "sounds good, keep going");
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn owner_restart_question_is_discussion() {
    let m = msg(
        "owner:human",
        "do I need to restart everything or just bpmux?",
    );
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn owner_path_to_screenshot_is_discussion() {
    let m = msg("owner:human", "/tmp/screenshot.png");
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn owner_keep_going_is_discussion() {
    let m = msg("owner:human", "great, keep going");
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn owner_yes_is_discussion() {
    let m = msg("owner:human", "yes");
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn agent_message_is_discussion() {
    let m = msg("dev:claude-code", "I'll fix the filter now");
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn agent_story_decision_is_decision() {
    let m = msg("pm:claude-code", "#92 accepted and closed");
    assert_eq!(classify(&m, "owner"), MessageClass::Decision);
}

#[test]
fn system_message_is_routine() {
    let m = Message {
        from: "bpmux".to_string(),
        text: "dev:ws joined".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    };
    assert_eq!(classify(&m, "owner"), MessageClass::Routine);
}

#[test]
fn agent_teaching_is_experience() {
    let m = msg(
        "dev:claude-code",
        "you should use the Result type for error handling here",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Teaching)
    );
}

#[test]
fn agent_question_is_experience() {
    let m = msg(
        "pm:claude-code",
        "have you considered splitting this into two stories?",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Question)
    );
}

#[test]
fn short_message_not_experience() {
    let m = msg("owner:human", "don't do it");
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion,);
}

#[test]
fn status_report_with_formatting_is_discussion() {
    let m = msg(
        "pm:claude-code",
        "**Ready:** #119 (side chat)\n**Backlog:** #82 (website)\nWhat should we focus on next?",
    );
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn bpmux_system_message_is_routine() {
    let m = msg("bpmux", "New build detected — reloading in 3 seconds...");
    assert_eq!(classify(&m, "owner"), MessageClass::Routine);
}

#[test]
fn coordination_message_is_discussion() {
    let m = msg(
        "dev:claude-code",
        "Starting on the false positive fix — the classifier is too broad",
    );
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn delivery_handoff_is_discussion() {
    let m = msg(
        "dev:claude-code",
        "PM, delivered #122. Desk check when ready.",
    );
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn status_update_with_should_is_discussion() {
    let m = msg(
        "pm:claude-code",
        "Dev should pick up #123 next. The ready queue has two stories.",
    );
    assert_eq!(classify(&m, "owner"), MessageClass::Discussion);
}

#[test]
fn is_experience_helper() {
    assert!(MessageClass::Experience(ExperienceKind::Correction).is_experience());
    assert!(MessageClass::Experience(ExperienceKind::Question).is_experience());
    assert!(!MessageClass::Discussion.is_experience());
    assert!(!MessageClass::Decision.is_experience());
}

#[test]
fn admission_youre_right_is_correction() {
    let m = msg("pm:claude-code", "you're right, my bad");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_my_call_is_correction() {
    let m = msg(
        "pm:claude-code",
        "my call — i assigned blame without checking the diff",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_my_miss_is_correction() {
    let m = msg("dev:claude-code", "my miss, i should have read the commit");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_my_bad_is_correction() {
    let m = msg("designer:claude-code", "my bad.");
    assert!(
        m.text.len() < MIN_EXPERIENCE_LEN,
        "admission input must stay under MIN_EXPERIENCE_LEN to exercise the length-gate bypass"
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_fair_point_is_correction() {
    let m = msg("dev:claude-code", "fair point, let me reconsider");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_good_catch_is_correction() {
    let m = msg("pm:claude-code", "good catch — fixing now");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_my_mistake_is_correction() {
    let m = msg(
        "dev:claude-code",
        "my mistake, i shouldn't have shipped that",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_wont_do_it_again_is_correction() {
    let m = msg("pm:claude-code", "won't do it again, adding the check");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_from_owner_also_classifies() {
    let m = msg("owner:human", "you're right, that was the wrong framing");
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn diplomatic_without_checking_is_correction() {
    let m = msg(
        "dev:claude-code",
        "you assigned blame without checking the actual diff",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn diplomatic_please_verify_is_correction() {
    let m = msg(
        "dev:claude-code",
        "please verify before attributing the regression to this commit",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}

#[test]
fn admission_not_a_false_positive_inside_longer_message() {
    let m = msg(
        "pm:claude-code",
        "ok i went back and looked at the diff, you're right about that one — shipping the fix",
    );
    assert_eq!(
        classify(&m, "owner"),
        MessageClass::Experience(ExperienceKind::Correction)
    );
}
