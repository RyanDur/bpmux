use crate::session::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageClass {
    Correction,
    Decision,
    Discussion,
    Routine,
}

const CORRECTION_SIGNALS: &[&str] = &[
    "don't do",
    "dont do",
    "stop doing",
    "you should",
    "we should",
    "remember,",
    "are you sure",
    "not that way",
    "that's wrong",
    "do you know the",
    "do you know why",
    "is that right",
    "why would",
    "why did you",
    "why are you",
    "the problem is",
    "go through",
    "focus on",
    "quality is",
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

pub fn classify(msg: &Message, owner_name: &str) -> MessageClass {
    if !msg.is_chat() {
        return MessageClass::Routine;
    }

    let sender = msg.from.split(':').next().unwrap_or(&msg.from);
    let text_lower = msg.text.to_lowercase();

    if sender == owner_name {
        let is_operational = OPERATIONAL_PATTERNS.iter().any(|p| text_lower.contains(p));
        if is_operational {
            return MessageClass::Discussion;
        }

        let has_correction_signal = CORRECTION_SIGNALS.iter().any(|s| text_lower.contains(s));
        if has_correction_signal && msg.text.len() > 20 {
            return MessageClass::Correction;
        }

        if DECISION_SIGNALS.iter().any(|s| text_lower.contains(s)) {
            return MessageClass::Decision;
        }
    }

    if DECISION_SIGNALS.iter().any(|s| text_lower.contains(s))
        && (text_lower.contains('#') || text_lower.contains("story"))
    {
        return MessageClass::Decision;
    }

    MessageClass::Discussion
}

pub fn classify_messages(messages: &[Message], owner_name: &str) -> Vec<MessageClass> {
    messages.iter().map(|m| classify(m, owner_name)).collect()
}
