use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BoardState {
    Icebox,
    Ready,
    InProgress,
    Acceptance,
    Accepted,
    Retired,
}

impl Serialize for BoardState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BoardState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        BoardState::parse(&raw)
            .ok_or_else(|| de::Error::custom(format!("unknown story state: {raw}")))
    }
}

impl BoardState {
    pub fn as_str(&self) -> &'static str {
        match self {
            BoardState::Icebox => "icebox",
            BoardState::Ready => "ready",
            BoardState::InProgress => "in-progress",
            BoardState::Acceptance => "acceptance",
            BoardState::Accepted => "accepted",
            BoardState::Retired => "retired",
        }
    }

    pub fn github_label(&self) -> &'static str {
        match self {
            BoardState::Icebox => "icebox",
            BoardState::Ready => "ready",
            BoardState::InProgress => "in progress",
            BoardState::Acceptance => "delivered",
            BoardState::Accepted => "accepted",
            BoardState::Retired => "retired",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "icebox" => Some(BoardState::Icebox),
            "ready" => Some(BoardState::Ready),
            "in-progress" | "in_progress" | "desk-check" | "desk_check" => {
                Some(BoardState::InProgress)
            }
            "acceptance" | "delivered" => Some(BoardState::Acceptance),
            "accepted" => Some(BoardState::Accepted),
            "retired" => Some(BoardState::Retired),
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

    pub fn parse(raw: &str) -> Option<Acceptor> {
        match raw {
            "team" => Some(Acceptor::Team),
            "owner" => Some(Acceptor::Owner),
            "dev" => Some(Acceptor::Dev),
            _ => None,
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
    DeskCheckFail,
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
