use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PeerId(String);

impl PeerId {
    pub fn local() -> Self {
        PeerId("local".to_string())
    }

    pub fn named(s: impl Into<String>) -> Self {
        PeerId(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
