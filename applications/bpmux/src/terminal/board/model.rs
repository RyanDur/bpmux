/// Story state as shown in the board columns (#193).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryState {
    Ready,
    InProgress,
    Delivered,
    Accepted,
    Deferred,
    Icebox,
}

impl StoryState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::InProgress => "IN PROGRESS",
            Self::Delivered => "DELIVERED",
            Self::Accepted => "ACCEPTED",
            Self::Deferred => "DEFERRED",
            Self::Icebox => "ICEBOX",
        }
    }

    /// Key used in the priority.json file.
    pub fn priority_key(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::InProgress => "in_progress",
            Self::Delivered => "delivered",
            Self::Accepted => "accepted",
            Self::Deferred => "deferred",
            Self::Icebox => "icebox",
        }
    }
}

/// A story on the board, parsed from a GitHub issue.
#[derive(Debug, Clone)]
pub struct BoardStory {
    pub number: u64,
    pub title: String,
    pub state: StoryState,
    pub body: String,
    pub assignees: Vec<String>,
    pub created: String,
    /// For deferred stories, the reason (extracted from label description or body).
    pub reason: String,
    pub labels: Vec<String>,
}

impl BoardStory {
    /// Short display name: `#NNN title` truncated to fit. When the
    /// story has a Feature/Bug/Chore label but the title doesn't
    /// already carry a `[Type]` prefix (pre-#264 filings that got
    /// label-backfilled but not title-migrated), the type is inserted
    /// so the display answers "what kind of story is this?" without
    /// the user needing to inspect GitHub (#278).
    pub fn short_name(&self, max_len: usize) -> String {
        use crate::session::velocity::{StoryType, classify_story};
        let prefix = format!("#{} ", self.number);
        let title_has_type_prefix = {
            let lower = self.title.to_ascii_lowercase();
            lower.starts_with("[feature")
                || lower.starts_with("[bug")
                || lower.starts_with("[chore")
        };
        let has_type_label = self
            .labels
            .iter()
            .any(|l| matches!(l.as_str(), "Feature" | "Bug" | "Chore" | "Epic"));
        let display_title = if !title_has_type_prefix && has_type_label {
            let type_tag = match classify_story(&self.labels, &self.title) {
                StoryType::Feature => "[Feature] ",
                StoryType::Bug => "[Bug] ",
                StoryType::Chore => "[Chore] ",
                StoryType::Epic => "[Epic] ",
            };
            format!("{type_tag}{}", self.title)
        } else {
            self.title.clone()
        };
        let budget = max_len.saturating_sub(prefix.len());
        if display_title.len() > budget {
            format!(
                "{prefix}{}…",
                display_title
                    .chars()
                    .take(budget.saturating_sub(1))
                    .collect::<String>()
            )
        } else {
            format!("{prefix}{display_title}")
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn story(number: u64, title: &str, state: StoryState, labels: &[&str]) -> BoardStory {
        BoardStory {
            number,
            title: title.to_string(),
            state,
            body: String::new(),
            assignees: Vec::new(),
            created: "2026-04-17".to_string(),
            reason: String::new(),
            labels: labels.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn short_name_prepends_type_when_label_present_but_title_lacks_prefix() {
        // The #278 core case: pre-#264 filings that got label-only
        // backfill. Display now surfaces the type from the label.
        let s = story(
            224,
            "As Ryan, I want click-to-focus",
            StoryState::Ready,
            &["Feature", "ready"],
        );
        let name = s.short_name(80);
        assert!(
            name.contains("[Feature]"),
            "Feature label should render as [Feature] prefix. got: {name}"
        );
    }

    #[test]
    fn short_name_does_not_double_tag_when_title_already_prefixed() {
        // Post-#264 filings include the [Type] prefix in the title.
        // Don't render [Feature] [Feature] X — idempotent.
        let s = story(
            275,
            "[Feature] As Ryan, I want threads",
            StoryState::Ready,
            &["Feature", "ready"],
        );
        let name = s.short_name(80);
        assert_eq!(
            name.matches("[Feature]").count(),
            1,
            "prefix should not duplicate. got: {name}"
        );
    }

    #[test]
    fn short_name_renders_bug_from_label() {
        let s = story(
            217,
            "opening board multiple times locks input",
            StoryState::Ready,
            &["Bug"],
        );
        let name = s.short_name(80);
        assert!(name.contains("[Bug]"), "got: {name}");
    }

    #[test]
    fn short_name_renders_chore_from_label() {
        let s = story(
            276,
            "widen integration-test timeouts",
            StoryState::Ready,
            &["Chore"],
        );
        let name = s.short_name(80);
        assert!(name.contains("[Chore]"), "got: {name}");
    }

    #[test]
    fn short_name_without_type_label_renders_unchanged() {
        // No Feature/Bug/Chore label → no prefix added.
        // This guards against classifier-default-to-Feature sneaking
        // an unlabeled story a phantom type.
        let s = story(999, "untyped legacy story", StoryState::Icebox, &["ready"]);
        let name = s.short_name(80);
        assert!(!name.contains("[Feature]"));
        assert!(!name.contains("[Bug]"));
        assert!(!name.contains("[Chore]"));
    }

    #[test]
    fn short_name_truncates_within_budget_after_prefix_insert() {
        let s = story(
            224,
            "very long user story title that will overflow the rendering budget",
            StoryState::Ready,
            &["Feature"],
        );
        let name = s.short_name(30);
        assert!(name.chars().count() <= 30);
        assert!(name.contains("[Feature]"));
        assert!(name.ends_with('…'));
    }
}
