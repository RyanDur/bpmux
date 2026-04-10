//! Message classification using Chain of Responsibility with
//! Aho-Corasick multi-pattern matching for O(m) classification.

use crate::session::Message;
use aho_corasick::AhoCorasick;
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

const MIN_EXPERIENCE_LEN: usize = 20;

pub const COORDINATION_PATTERNS: &[&str] = &[
    "starting on",
    "working on",
    "picking up",
    "delivered",
    "desk check",
    "ready for",
    "available for",
    "standing by",
    "on it",
    "pushed",
    "committed",
    "restating",
    "waiting for",
    "moving to",
    "switching to",
    "current state",
    "status check",
    "matches",
    "sign-off",
    "accepted",
    "clean clippy",
    "tests pass",
];

const OPERATIONAL: &[&str] = &[
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

const DECISION: &[&str] = &[
    "accepted", "rejected", "closed", "filed", "accept", "ship it", "approved",
];

const REPORT_INDICATORS: &[&str] = &["**", "##", "- ", "* ", "1. ", "2. ", "3. "];

const CORRECTION: &[&str] = &[
    "don't do",
    "dont do",
    "stop doing",
    "not that way",
    "that's wrong",
    "the problem is",
];

const QUESTION: &[&str] = &[
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

const TEACHING: &[&str] = &[
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

const REDIRECT: &[&str] = &[
    "not that",
    "instead,",
    "rather,",
    "actually,",
    "let me clarify",
    "what i meant",
    "no,",
];

fn matches(matcher: &Option<AhoCorasick>, text: &str) -> bool {
    matcher.as_ref().is_some_and(|m| m.is_match(text))
}

fn build_matcher(patterns: &[&str]) -> Option<AhoCorasick> {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(patterns)
        .ok()
}

/// Pre-built pattern matchers for O(m) classification.
pub struct Classifier {
    operational: Option<AhoCorasick>,
    decision: Option<AhoCorasick>,
    coordination: Option<AhoCorasick>,
    correction: Option<AhoCorasick>,
    question: Option<AhoCorasick>,
    teaching: Option<AhoCorasick>,
    redirect: Option<AhoCorasick>,
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

/// Convenience function that creates a temporary classifier.
/// For hot paths, use Classifier::new() once and reuse.
pub fn classify(msg: &Message, owner_name: &str) -> MessageClass {
    // Use a thread-local cached classifier to avoid rebuilding on every call
    thread_local! {
        static CLASSIFIER: Classifier = Classifier::new();
    }
    CLASSIFIER.with(|c| c.classify(msg, owner_name))
}

pub fn classify_messages(messages: &[Message], owner_name: &str) -> Vec<MessageClass> {
    let c = Classifier::new();
    messages.iter().map(|m| c.classify(m, owner_name)).collect()
}
