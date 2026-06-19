use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::session::Message;
use crate::session::classifier::{ExperienceKind, MessageClass};

const SMOOTHING: f64 = 1.0;

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

        if !msg.is_chat() {
            return MessageClass::Routine;
        }
        let sender = msg.from.split(':').next().unwrap_or(&msg.from);
        if sender == "bpmux" {
            return MessageClass::Routine;
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

    if sender == owner_name {
        features.push("sender:owner".into());
    } else if msg.from.contains("claude-code") {
        features.push("sender:agent".into());
    } else {
        features.push("sender:other".into());
    }

    let len = text.len();
    if len < 20 {
        features.push("len:short".into());
    } else if len < 80 {
        features.push("len:medium".into());
    } else {
        features.push("len:long".into());
    }

    if text.contains('?') {
        features.push("punct:question".into());
    }
    if text.contains('!') {
        features.push("punct:exclaim".into());
    }

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

pub(crate) fn class_label(class: &MessageClass) -> String {
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

pub(crate) fn parse_class_label(label: &str) -> MessageClass {
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
