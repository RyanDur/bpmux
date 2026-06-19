use super::bayes::*;
use crate::session::Message;
use crate::session::classifier::{ExperienceKind, MessageClass};
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
fn untrained_model_falls_back_to_keyword_classifier() {
    let model = BayesModel::default();
    assert!(!model.is_trained());
    let m = msg("owner:human", "you should restate before building");
    let result = model.classify(&m, "owner");
    assert!(result.is_experience());
}

#[test]
fn trained_model_classifies() {
    let mut model = BayesModel::default();
    let training_data = vec![
        msg("owner:human", "you should focus on the problem not the fix"),
        msg("owner:human", "you should restate before building anything"),
        msg("owner:human", "you should always verify before showing me"),
        msg(
            "owner:human",
            "why did you pick that approach over the simpler one?",
        ),
        msg(
            "owner:human",
            "why are you not available? what is preventing you?",
        ),
        msg("owner:human", "do you know why we test through public API?"),
        msg("owner:human", "don't do that, it breaks the build process"),
        msg(
            "owner:human",
            "don't do this just for me, do it for each other",
        ),
        msg("owner:human", "remember, quality is the goal in everything"),
        msg(
            "owner:human",
            "focus on the problem, make sure you understand it",
        ),
        msg(
            "owner:human",
            "all design feedback should go through the designer",
        ),
        msg(
            "dev:claude-code",
            "you should use the Result type for error handling here",
        ),
        msg(
            "pm:claude-code",
            "have you considered splitting this into two stories?",
        ),
        msg("dev:claude-code", "I'll fix the filter now"),
        msg("dev:claude-code", "understood, working on it"),
        msg("dev:claude-code", "tests pass, pushing now"),
        msg("owner:human", "sounds good, keep going"),
        msg("owner:human", "great, continue with work"),
        msg("owner:human", "ok"),
        msg("owner:human", "yes"),
        msg("pm:claude-code", "board updated, three stories in ready"),
        msg("designer:claude-code", "design sign-off on the overlay"),
        msg("pm:claude-code", "#92 accepted and closed"),
        msg("owner:human", "accepted"),
        msg("pm:claude-code", "#110 filed and in backlog"),
    ];
    model.train(&training_data, "owner");
    assert!(model.is_trained());

    let m = msg(
        "owner:human",
        "you should always verify before showing me the result",
    );
    let result = model.classify(&m, "owner");
    assert!(
        result.is_experience(),
        "expected experience, got {result:?}"
    );
}

#[test]
fn class_label_roundtrips() {
    let classes = vec![
        MessageClass::Experience(ExperienceKind::Correction),
        MessageClass::Experience(ExperienceKind::Question),
        MessageClass::Experience(ExperienceKind::Teaching),
        MessageClass::Experience(ExperienceKind::Redirect),
        MessageClass::Decision,
        MessageClass::Discussion,
        MessageClass::Routine,
    ];
    for class in classes {
        let label = class_label(&class);
        assert_eq!(parse_class_label(&label), class);
    }
}
