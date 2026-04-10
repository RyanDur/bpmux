use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::session::Message;
use crate::session::classifier::{ExperienceKind, MessageClass};

const SMOOTHING: f64 = 1.0; // Laplace smoothing

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BayesModel {
    class_counts: HashMap<String, usize>,
    feature_counts: HashMap<String, HashMap<String, usize>>,
    total: usize,
}

impl BayesModel {
    pub fn is_trained(&self) -> bool {
        self.total > 0
    }

    pub fn train(&mut self, messages: &[Message], owner_name: &str) {
        self.class_counts.clear();
        self.feature_counts.clear();
        self.total = 0;

        for msg in messages {
            let class = super::classifier::classify(msg, owner_name);
            let label = class_label(&class);
            let features = extract_features(msg, owner_name);

            *self.class_counts.entry(label.clone()).or_default() += 1;
            self.total += 1;

            let feature_map = self.feature_counts.entry(label).or_default();
            for feature in features {
                *feature_map.entry(feature).or_default() += 1;
            }
        }
    }

    pub fn classify(&self, msg: &Message, owner_name: &str) -> MessageClass {
        if !self.is_trained() {
            return super::classifier::classify(msg, owner_name);
        }

        let features = extract_features(msg, owner_name);
        let num_features = count_unique_features(&self.feature_counts);

        let mut best_class = "discussion".to_string();
        let mut best_score = f64::NEG_INFINITY;

        for (class, &count) in &self.class_counts {
            let prior = (count as f64).ln() - (self.total as f64).ln();
            let class_features = self.feature_counts.get(class);

            let likelihood: f64 = features
                .iter()
                .map(|f| {
                    let feat_count =
                        class_features.and_then(|m| m.get(f)).copied().unwrap_or(0) as f64;
                    ((feat_count + SMOOTHING) / (count as f64 + SMOOTHING * num_features as f64))
                        .ln()
                })
                .sum();

            let score = prior + likelihood;
            if score > best_score {
                best_score = score;
                best_class = class.clone();
            }
        }

        parse_class_label(&best_class)
    }
}

fn extract_features(msg: &Message, owner_name: &str) -> Vec<String> {
    let sender = msg.from.split(':').next().unwrap_or(&msg.from);
    let text = &msg.text;
    let text_lower = text.to_lowercase();

    let mut features = Vec::new();

    // Sender features
    if sender == owner_name {
        features.push("sender:owner".into());
    } else if msg.from.contains("claude-code") {
        features.push("sender:agent".into());
    } else {
        features.push("sender:other".into());
    }

    // Length features
    let len = text.len();
    if len < 20 {
        features.push("len:short".into());
    } else if len < 80 {
        features.push("len:medium".into());
    } else {
        features.push("len:long".into());
    }

    // Punctuation features
    if text.contains('?') {
        features.push("punct:question".into());
    }
    if text.contains('!') {
        features.push("punct:exclaim".into());
    }

    // Word pattern features
    let signal_words: &[(&str, &str)] = &[
        ("don't", "word:dont"),
        ("should", "word:should"),
        ("why", "word:why"),
        ("how", "word:how"),
        ("remember", "word:remember"),
        ("focus", "word:focus"),
        ("quality", "word:quality"),
        ("instead", "word:instead"),
        ("actually", "word:actually"),
        ("consider", "word:consider"),
        ("think", "word:think"),
        ("know", "word:know"),
        ("right", "word:right"),
        ("wrong", "word:wrong"),
        ("problem", "word:problem"),
        ("reason", "word:reason"),
        ("because", "word:because"),
    ];

    for (word, feature) in signal_words {
        if text_lower.contains(word) {
            features.push((*feature).into());
        }
    }

    // Addressing features
    if msg.to.is_none() {
        features.push("target:broadcast".into());
    } else {
        features.push("target:directed".into());
    }

    features
}

fn count_unique_features(feature_counts: &HashMap<String, HashMap<String, usize>>) -> usize {
    let mut all_features: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for class_features in feature_counts.values() {
        for feature in class_features.keys() {
            all_features.insert(feature);
        }
    }
    all_features.len().max(1)
}

fn class_label(class: &MessageClass) -> String {
    match class {
        MessageClass::Experience(ExperienceKind::Correction) => "experience:correction".into(),
        MessageClass::Experience(ExperienceKind::Question) => "experience:question".into(),
        MessageClass::Experience(ExperienceKind::Teaching) => "experience:teaching".into(),
        MessageClass::Experience(ExperienceKind::Redirect) => "experience:redirect".into(),
        MessageClass::Decision => "decision".into(),
        MessageClass::Discussion => "discussion".into(),
        MessageClass::Routine => "routine".into(),
    }
}

fn parse_class_label(label: &str) -> MessageClass {
    match label {
        "experience:correction" => MessageClass::Experience(ExperienceKind::Correction),
        "experience:question" => MessageClass::Experience(ExperienceKind::Question),
        "experience:teaching" => MessageClass::Experience(ExperienceKind::Teaching),
        "experience:redirect" => MessageClass::Experience(ExperienceKind::Redirect),
        "decision" => MessageClass::Decision,
        "routine" => MessageClass::Routine,
        _ => MessageClass::Discussion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::message::MessageType;
    use chrono::Utc;

    fn msg(from: &str, text: &str) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to: None,
            addressee: None,
            explicit_privacy: false,
        }
    }

    #[test]
    fn untrained_model_falls_back_to_keyword_classifier() {
        let model = BayesModel::default();
        assert!(!model.is_trained());
        let m = msg("ryan:human", "you should restate before building");
        let result = model.classify(&m, "ryan");
        assert!(result.is_experience());
    }

    #[test]
    fn trained_model_classifies() {
        let mut model = BayesModel::default();
        let training_data = vec![
            // Experiences
            msg("ryan:human", "you should focus on the problem not the fix"),
            msg("ryan:human", "you should restate before building anything"),
            msg("ryan:human", "you should always verify before showing me"),
            msg(
                "ryan:human",
                "why did you pick that approach over the simpler one?",
            ),
            msg(
                "ryan:human",
                "why are you not available? what is preventing you?",
            ),
            msg("ryan:human", "do you know why we test through public API?"),
            msg("ryan:human", "don't do that, it breaks the build process"),
            msg(
                "ryan:human",
                "don't do this just for me, do it for each other",
            ),
            msg("ryan:human", "remember, quality is the goal in everything"),
            msg(
                "ryan:human",
                "focus on the problem, make sure you understand it",
            ),
            msg(
                "ryan:human",
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
            // Discussions
            msg("dev:claude-code", "I'll fix the filter now"),
            msg("dev:claude-code", "understood, working on it"),
            msg("dev:claude-code", "tests pass, pushing now"),
            msg("ryan:human", "sounds good, keep going"),
            msg("ryan:human", "great, continue with work"),
            msg("ryan:human", "ok"),
            msg("ryan:human", "yes"),
            msg("pm:claude-code", "board updated, three stories in ready"),
            msg("designer:claude-code", "design sign-off on the overlay"),
            // Decisions
            msg("pm:claude-code", "#92 accepted and closed"),
            msg("ryan:human", "accepted"),
            msg("pm:claude-code", "#110 filed and in backlog"),
        ];
        model.train(&training_data, "ryan");
        assert!(model.is_trained());

        // Teaching message from owner should be experience
        let m = msg(
            "ryan:human",
            "you should always verify before showing me the result",
        );
        let result = model.classify(&m, "ryan");
        assert!(
            result.is_experience(),
            "expected experience, got {result:?}"
        );
    }

    #[test]
    fn extract_features_captures_sender_and_patterns() {
        let m = msg("ryan:human", "why did you do that? you should think first");
        let features = extract_features(&m, "ryan");
        assert!(features.contains(&"sender:owner".to_string()));
        assert!(features.contains(&"punct:question".to_string()));
        assert!(features.contains(&"word:why".to_string()));
        assert!(features.contains(&"word:should".to_string()));
        assert!(features.contains(&"word:think".to_string()));
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
}
