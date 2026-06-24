use crate::message::Message;
use aho_corasick::AhoCorasick;
use serde::{Deserialize, Serialize};

mod patterns;

pub use patterns::COORDINATION_PATTERNS;
pub(crate) use patterns::MIN_EXPERIENCE_LEN;

use patterns::{
    ADMISSION, CORRECTION, DECISION, OPERATIONAL, QUESTION, REDIRECT, REPORT_INDICATORS, TEACHING,
};

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

fn matches(matcher: &Option<AhoCorasick>, text: &str) -> bool {
    matcher.as_ref().is_some_and(|m| m.is_match(text))
}

fn build_matcher(patterns: &[&str]) -> Option<AhoCorasick> {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(patterns)
        .ok()
}

pub struct Classifier {
    operational: Option<AhoCorasick>,
    decision: Option<AhoCorasick>,
    coordination: Option<AhoCorasick>,
    correction: Option<AhoCorasick>,
    question: Option<AhoCorasick>,
    teaching: Option<AhoCorasick>,
    redirect: Option<AhoCorasick>,
    admission: Option<AhoCorasick>,
}

impl Default for Classifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Classifier {
    pub fn new() -> Self {
        Self {
            operational: build_matcher(OPERATIONAL),
            decision: build_matcher(DECISION),
            coordination: build_matcher(COORDINATION_PATTERNS),
            correction: build_matcher(CORRECTION),
            question: build_matcher(QUESTION),
            teaching: build_matcher(TEACHING),
            redirect: build_matcher(REDIRECT),
            admission: build_matcher(ADMISSION),
        }
    }

    pub fn classify(&self, msg: &Message, owner_name: &str) -> MessageClass {
        if !msg.is_chat() {
            return MessageClass::Routine;
        }

        let sender = msg.from.split(':').next().unwrap_or(&msg.from);
        if sender == "bpmux" {
            return MessageClass::Routine;
        }

        let text_lower = msg.text.to_lowercase();

        if matches(&self.admission, &text_lower) {
            return MessageClass::Experience(ExperienceKind::Correction);
        }

        if sender == owner_name && matches(&self.operational, &text_lower) {
            return MessageClass::Discussion;
        }

        if matches(&self.decision, &text_lower)
            && (sender == owner_name || text_lower.contains('#') || text_lower.contains("story"))
        {
            return MessageClass::Decision;
        }

        if matches(&self.coordination, &text_lower) {
            return MessageClass::Discussion;
        }

        if REPORT_INDICATORS.iter().any(|r| msg.text.contains(r)) {
            return MessageClass::Discussion;
        }

        if msg.text.len() >= MIN_EXPERIENCE_LEN {
            if matches(&self.correction, &text_lower) {
                return MessageClass::Experience(ExperienceKind::Correction);
            }
            if matches(&self.question, &text_lower) {
                return MessageClass::Experience(ExperienceKind::Question);
            }
            if matches(&self.teaching, &text_lower) {
                return MessageClass::Experience(ExperienceKind::Teaching);
            }
            if matches(&self.redirect, &text_lower) {
                return MessageClass::Experience(ExperienceKind::Redirect);
            }
        }

        MessageClass::Discussion
    }
}

pub fn classify(msg: &Message, owner_name: &str) -> MessageClass {
    thread_local! {
        static CLASSIFIER: Classifier = Classifier::new();
    }
    CLASSIFIER.with(|c| c.classify(msg, owner_name))
}

pub fn classify_messages(messages: &[Message], owner_name: &str) -> Vec<MessageClass> {
    messages.iter().map(|m| classify(m, owner_name)).collect()
}
