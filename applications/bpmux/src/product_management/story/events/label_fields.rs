use serde::{Deserialize, Serialize};

use crate::product_management::epic::EpicId;
use crate::product_management::story::{Acceptor, IterationSlot, Tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LabelDerivedFields {
    pub raw_labels: Vec<String>,
    #[serde(default)]
    pub acceptor: Option<Acceptor>,
    #[serde(default)]
    pub parent_epic: Option<EpicId>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub iteration_slot: Option<IterationSlot>,
}
