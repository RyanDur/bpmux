use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMeta {
    pub actor: String,
    #[serde(default)]
    pub actor_id: Option<String>,
    pub ts: DateTime<Utc>,
}
