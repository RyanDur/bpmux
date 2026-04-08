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
    "don't",
    "dont",
    "stop",
    "should",
    "why ",
    "why?",
    "remember",
    "are you sure",
    "not that",
    "wrong",
    "what if",
    "do you know",
    "is that right",
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
        let has_correction_signal = CORRECTION_SIGNALS.iter().any(|s| text_lower.contains(s));
        let has_substantive_question = msg.text.contains('?') && msg.text.len() > 30;
        if has_correction_signal || has_substantive_question {
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
