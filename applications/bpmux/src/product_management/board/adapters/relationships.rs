use serde::{Deserialize, Serialize};

use super::relationship_block::{embed, extract};
use crate::product_management::story::StoryId;

pub trait Relationships: Serialize + serde::de::DeserializeOwned + Sized {
    fn read_from_body(body: &str) -> Option<Self> {
        serde_json::from_value(extract(body)?).ok()
    }

    fn write_into_body(&self, body: &str) -> String {
        match serde_json::to_value(self) {
            Ok(value) => embed(body, &value),
            Err(_) => body.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StoryRelationships {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epic: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IterationRelationships {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stories: Vec<StoryId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OutcomeRelationships {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roadmap: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub iterations: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RoadmapRelationships {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EpicRelationships {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stories: Vec<StoryId>,
}

impl Relationships for StoryRelationships {}
impl Relationships for IterationRelationships {}
impl Relationships for OutcomeRelationships {}
impl Relationships for RoadmapRelationships {}
impl Relationships for EpicRelationships {}
