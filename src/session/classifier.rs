use crate::session::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExperienceKind {
    Correction,
    Question,
    Teaching,
    Redirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageClass {
    Experience(ExperienceKind),
    Decision,
    Discussion,
    Routine,
}

impl MessageClass {
    pub fn is_experience(&self) -> bool {
        matches!(self, MessageClass::Experience(_))
    }
}

const CORRECTION_SIGNALS: &[&str] = &[
    "don't do",
    "dont do",
    "stop doing",
    "not that way",
    "that's wrong",
    "the problem is",
];

const QUESTION_SIGNALS: &[&str] = &[
    "do you know",
    "are you sure",
    "is that right",
    "why would",
    "why did you",
    "why are you",
    "what do you think",
    "have you considered",
    "what if",
    "how would you",
];

const TEACHING_SIGNALS: &[&str] = &[
    "you should",
    "we should",
    "remember,",
    "the reason is",
    "this is because",
    "the way to",
    "focus on",
    "quality is",
    "go through",
    "the right way",
];

const REDIRECT_SIGNALS: &[&str] = &[
    "not that",
    "instead,",
    "rather,",
    "actually,",
    "let me clarify",
    "what i meant",
    "no,",
];

const OPERATIONAL_PATTERNS: &[&str] = &[
    "restart",
    "rebuild",
    "see me",
    "you in?",
    "see this",
    "screen-shots",
    "img_",
    "how about now",
    "hows it going",
    "hows it goin",
    "everything ok",
    "something wrong",
    "need anything",
    "keep going",
    "sounds good",
    "great,",
    "got it",
    "restarted",
    "rebuilt",
];

const DECISION_SIGNALS: &[&str] = &[
    "accepted", "rejected", "closed", "filed", "accept", "ship it", "approved",
];

const MIN_EXPERIENCE_LEN: usize = 20;

const REPORT_INDICATORS: &[&str] = &["**", "##", "- ", "* ", "1. ", "2. ", "3. "];

pub fn classify(msg: &Message, owner_name: &str) -> MessageClass {
    if !msg.is_chat() {
        return MessageClass::Routine;
    }

    let sender = msg.from.split(':').next().unwrap_or(&msg.from);
    if sender == "bpmux" {
        return MessageClass::Routine;
    }

    let text_lower = msg.text.to_lowercase();

    let is_operational = OPERATIONAL_PATTERNS.iter().any(|p| text_lower.contains(p));
    if is_operational && sender == owner_name {
        return MessageClass::Discussion;
    }

    // Status reports with markdown formatting are not experiences
    if REPORT_INDICATORS.iter().any(|r| msg.text.contains(r)) {
        if DECISION_SIGNALS.iter().any(|s| text_lower.contains(s))
            && (sender == owner_name || text_lower.contains('#') || text_lower.contains("story"))
        {
            return MessageClass::Decision;
        }
        return MessageClass::Discussion;
    }

    if msg.text.len() >= MIN_EXPERIENCE_LEN {
        if CORRECTION_SIGNALS.iter().any(|s| text_lower.contains(s)) {
            return MessageClass::Experience(ExperienceKind::Correction);
        }
        if QUESTION_SIGNALS.iter().any(|s| text_lower.contains(s)) {
            return MessageClass::Experience(ExperienceKind::Question);
        }
        if TEACHING_SIGNALS.iter().any(|s| text_lower.contains(s)) {
            return MessageClass::Experience(ExperienceKind::Teaching);
        }
        if REDIRECT_SIGNALS.iter().any(|s| text_lower.contains(s)) {
            return MessageClass::Experience(ExperienceKind::Redirect);
        }
    }

    if DECISION_SIGNALS.iter().any(|s| text_lower.contains(s))
        && (sender == owner_name || text_lower.contains('#') || text_lower.contains("story"))
    {
        return MessageClass::Decision;
    }

    MessageClass::Discussion
}

pub fn classify_messages(messages: &[Message], owner_name: &str) -> Vec<MessageClass> {
    messages.iter().map(|m| classify(m, owner_name)).collect()
}
