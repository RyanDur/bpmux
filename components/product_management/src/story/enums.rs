use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Work {
    Ready,
    InProgress,
    Acceptance,
    Accepted,
    Retired,
}

impl Serialize for Work {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Work {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Work::parse(&raw).ok_or_else(|| de::Error::custom(format!("unknown story state: {raw}")))
    }
}

impl Work {
    pub fn as_str(&self) -> &'static str {
        match self {
            Work::Ready => "ready",
            Work::InProgress => "in-progress",
            Work::Acceptance => "acceptance",
            Work::Accepted => "accepted",
            Work::Retired => "retired",
        }
    }

    pub fn github_label(&self) -> Option<&'static str> {
        match self {
            Work::Ready => None,
            Work::InProgress => Some("in progress"),
            Work::Acceptance => Some("delivered"),
            Work::Accepted => Some("accepted"),
            Work::Retired => Some("retired"),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ready" | "not-started" | "icebox" => Some(Work::Ready),
            "in-progress" | "in_progress" | "desk-check" | "desk_check" => Some(Work::InProgress),
            "acceptance" | "delivered" => Some(Work::Acceptance),
            "accepted" => Some(Work::Accepted),
            "retired" => Some(Work::Retired),
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
