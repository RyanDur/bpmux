pub(crate) const MIN_EXPERIENCE_LEN: usize = 20;

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

pub(super) const OPERATIONAL: &[&str] = &[
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

pub(super) const DECISION: &[&str] = &[
    "accepted", "rejected", "closed", "filed", "accept", "ship it", "approved",
];

pub(super) const REPORT_INDICATORS: &[&str] = &["**", "##", "- ", "* ", "1. ", "2. ", "3. "];

pub(super) const CORRECTION: &[&str] = &[
    "don't do",
    "dont do",
    "stop doing",
    "not that way",
    "that's wrong",
    "the problem is",
    "without checking",
    "please verify",
    "before attributing",
    "lazy causation",
];

pub(super) const ADMISSION: &[&str] = &[
    "you're right",
    "youre right",
    "my call",
    "my miss",
    "my bad",
    "my mistake",
    "fair point",
    "good catch",
    "won't do it again",
    "wont do it again",
];

pub(super) const QUESTION: &[&str] = &[
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

pub(super) const TEACHING: &[&str] = &[
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

pub(super) const REDIRECT: &[&str] = &[
    "not that",
    "instead,",
    "rather,",
    "actually,",
    "let me clarify",
    "what i meant",
    "no,",
];
