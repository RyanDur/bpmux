use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Dev,
    DeskCheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoryState {
    Icebox,
    Ready,
    InProgress(Stage),
    Delivered,
    Accepted,
    Retired,
}

impl Serialize for StoryState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StoryState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        StoryState::parse(&raw)
            .ok_or_else(|| de::Error::custom(format!("unknown story state: {raw}")))
    }
}

impl StoryState {
    pub fn as_str(self) -> &'static str {
        match self {
            StoryState::Icebox => "icebox",
            StoryState::Ready => "ready",
            StoryState::InProgress(Stage::Dev) => "in-progress",
            StoryState::InProgress(Stage::DeskCheck) => "desk-check",
            StoryState::Delivered => "delivered",
            StoryState::Accepted => "accepted",
            StoryState::Retired => "retired",
        }
    }

    pub fn github_label(self) -> &'static str {
        match self {
            StoryState::Icebox => "icebox",
            StoryState::Ready => "ready",
            StoryState::InProgress(Stage::Dev) => "in progress",
            StoryState::InProgress(Stage::DeskCheck) => "desk-check",
            StoryState::Delivered => "delivered",
            StoryState::Accepted => "accepted",
            StoryState::Retired => "retired",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "icebox" => Some(StoryState::Icebox),
            "ready" => Some(StoryState::Ready),
            "in-progress" | "in_progress" => Some(StoryState::InProgress(Stage::Dev)),
            "desk-check" | "desk_check" => Some(StoryState::InProgress(Stage::DeskCheck)),
            "delivered" => Some(StoryState::Delivered),
            "accepted" => Some(StoryState::Accepted),
            "retired" => Some(StoryState::Retired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Acceptor {
    Team,
    Owner,
    Dev,
}

impl Acceptor {
    pub fn as_str(self) -> &'static str {
        match self {
            Acceptor::Team => "team",
            Acceptor::Owner => "owner",
            Acceptor::Dev => "dev",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Horizon {
    Now,
    Next,
    Later,
}

impl Horizon {
    pub fn as_str(self) -> &'static str {
        match self {
            Horizon::Now => "now",
            Horizon::Next => "next",
            Horizon::Later => "later",
        }
    }

    pub fn band_label(self) -> &'static str {
        match self {
            Horizon::Now => "Now",
            Horizon::Next => "Next",
            Horizon::Later => "Later",
        }
    }

    pub fn from_cli(raw: &str) -> Option<Horizon> {
        match raw.to_ascii_lowercase().as_str() {
            "now" => Some(Horizon::Now),
            "next" => Some(Horizon::Next),
            "later" => Some(Horizon::Later),
            _ => None,
        }
    }

    pub const CLI_VALUES: &'static str = "now | next | later";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tag {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gate {
    Filed,
    Pickup,
    Delivered,
    PassDeskCheck,
    FailDeskCheck,
    Rejected,
    Accepted,
}
