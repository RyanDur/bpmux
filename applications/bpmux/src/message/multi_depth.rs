use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiDepthPayload {
    pub summary: String,

    #[serde(default)]
    pub role_default: std::collections::HashMap<String, String>,

    pub deep: DeepDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeepDetail {
    Preloaded {
        text: String,
    },

    OnDemand {
        command: String,
        description: String,
    },
}
