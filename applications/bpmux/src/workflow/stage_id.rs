use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct StageId(Arc<str>);

impl StageId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for StageId {
    fn from(s: &str) -> Self {
        StageId(Arc::from(s))
    }
}

impl From<String> for StageId {
    fn from(s: String) -> Self {
        StageId(Arc::from(s.as_str()))
    }
}

impl fmt::Display for StageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StageId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(StageId::from(raw))
    }
}
