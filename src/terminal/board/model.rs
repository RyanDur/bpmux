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
    /// Short display name: `#NNN title` truncated to fit.
    pub fn short_name(&self, max_len: usize) -> String {
        let prefix = format!("#{} ", self.number);
        let budget = max_len.saturating_sub(prefix.len());
        if self.title.len() > budget {
            format!(
                "{prefix}{}…",
                self.title
                    .chars()
                    .take(budget.saturating_sub(1))
                    .collect::<String>()
            )
        } else {
            format!("{prefix}{}", self.title)
        }
    }
}
