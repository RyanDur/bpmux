use serde::{Deserialize, Serialize};

use crate::epic::EpicId;
use crate::story::{Acceptor, Tag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LabelDerivedFields {
    pub raw_labels: Vec<String>,
    #[serde(default)]
    pub acceptor: Option<Acceptor>,
    #[serde(default)]
    pub parent_epic: Option<EpicId>,
    #[serde(default)]
    pub tags: Vec<Tag>,
}
