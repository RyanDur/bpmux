//! Message classification using Chain of Responsibility.
//!
//! Each handler examines a message and either classifies it or
//! passes to the next handler. The chain order defines priority.

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

/// Context passed through the classification chain.
pub struct ClassifyContext<'a> {
    pub msg: &'a Message,
    pub owner_name: &'a str,
    pub sender: &'a str,
    pub text_lower: String,
}

/// A handler in the classification chain. Returns Some to classify,
/// None to pass to the next handler.
type Handler = fn(&ClassifyContext) -> Option<MessageClass>;

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

/// The classification chain — order defines priority.
const CHAIN: &[Handler] = &[
    filter_non_chat,
    filter_system_sender,
    filter_owner_operational,
    detect_decision,
    filter_coordination,
    filter_report_formatting,
    detect_correction,
    detect_question,
    detect_teaching,
    detect_redirect,
];

pub fn classify(msg: &Message, owner_name: &str) -> MessageClass {
    let sender = msg.from.split(':').next().unwrap_or(&msg.from);
    let ctx = ClassifyContext {
        msg,
        owner_name,
        sender,
        text_lower: msg.text.to_lowercase(),
    };

    for handler in CHAIN {
        if let Some(class) = handler(&ctx) {
            return class;
        }
    }

    MessageClass::Discussion
}

pub fn classify_messages(messages: &[Message], owner_name: &str) -> Vec<MessageClass> {
    messages.iter().map(|m| classify(m, owner_name)).collect()
}

// --- Filters (return Discussion/Routine to exclude) ---

fn filter_non_chat(ctx: &ClassifyContext) -> Option<MessageClass> {
    if !ctx.msg.is_chat() {
        Some(MessageClass::Routine)
    } else {
        None
    }
}

fn filter_system_sender(ctx: &ClassifyContext) -> Option<MessageClass> {
    if ctx.sender == "bpmux" {
        Some(MessageClass::Routine)
    } else {
        None
    }
}

fn filter_owner_operational(ctx: &ClassifyContext) -> Option<MessageClass> {
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

    if ctx.sender == ctx.owner_name && OPERATIONAL.iter().any(|p| ctx.text_lower.contains(p)) {
        Some(MessageClass::Discussion)
    } else {
        None
    }
}

fn filter_coordination(ctx: &ClassifyContext) -> Option<MessageClass> {
    if COORDINATION_PATTERNS
        .iter()
        .any(|p| ctx.text_lower.contains(p))
    {
        Some(MessageClass::Discussion)
    } else {
        None
    }
}

fn filter_report_formatting(ctx: &ClassifyContext) -> Option<MessageClass> {
    const REPORT_INDICATORS: &[&str] = &["**", "##", "- ", "* ", "1. ", "2. ", "3. "];

    if REPORT_INDICATORS.iter().any(|r| ctx.msg.text.contains(r)) {
        Some(MessageClass::Discussion)
    } else {
        None
    }
}

// --- Detectors (return a classification when matched) ---

fn detect_decision(ctx: &ClassifyContext) -> Option<MessageClass> {
    const DECISION_SIGNALS: &[&str] = &[
        "accepted", "rejected", "closed", "filed", "accept", "ship it", "approved",
    ];

    if DECISION_SIGNALS.iter().any(|s| ctx.text_lower.contains(s))
        && (ctx.sender == ctx.owner_name
            || ctx.text_lower.contains('#')
            || ctx.text_lower.contains("story"))
    {
        Some(MessageClass::Decision)
    } else {
        None
    }
}

fn detect_correction(ctx: &ClassifyContext) -> Option<MessageClass> {
    const SIGNALS: &[&str] = &[
        "don't do",
        "dont do",
        "stop doing",
        "not that way",
        "that's wrong",
        "the problem is",
    ];

    if ctx.msg.text.len() >= MIN_EXPERIENCE_LEN
        && SIGNALS.iter().any(|s| ctx.text_lower.contains(s))
    {
        Some(MessageClass::Experience(ExperienceKind::Correction))
    } else {
        None
    }
}

fn detect_question(ctx: &ClassifyContext) -> Option<MessageClass> {
    const SIGNALS: &[&str] = &[
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

    if ctx.msg.text.len() >= MIN_EXPERIENCE_LEN
        && SIGNALS.iter().any(|s| ctx.text_lower.contains(s))
    {
        Some(MessageClass::Experience(ExperienceKind::Question))
    } else {
        None
    }
}

fn detect_teaching(ctx: &ClassifyContext) -> Option<MessageClass> {
    const SIGNALS: &[&str] = &[
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

    if ctx.msg.text.len() >= MIN_EXPERIENCE_LEN
        && SIGNALS.iter().any(|s| ctx.text_lower.contains(s))
    {
        Some(MessageClass::Experience(ExperienceKind::Teaching))
    } else {
        None
    }
}

fn detect_redirect(ctx: &ClassifyContext) -> Option<MessageClass> {
    const SIGNALS: &[&str] = &[
        "not that",
        "instead,",
        "rather,",
        "actually,",
        "let me clarify",
        "what i meant",
        "no,",
    ];

    if ctx.msg.text.len() >= MIN_EXPERIENCE_LEN
        && SIGNALS.iter().any(|s| ctx.text_lower.contains(s))
    {
        Some(MessageClass::Experience(ExperienceKind::Redirect))
    } else {
        None
    }
}
