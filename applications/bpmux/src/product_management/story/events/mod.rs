use serde::{Deserialize, Serialize};

use crate::product_management::story::StoryId;

mod accessors;
mod constructors;
mod kind;
mod label_fields;
mod meta;
mod wire;
mod wire_decode;
mod wire_encode;

pub use kind::StoryEventKind;
pub use label_fields::LabelDerivedFields;
pub use meta::{EventMeta, event_wins, resolve_order_key};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "wire::StoryEventWire", into = "wire::StoryEventWire")]
pub struct StoryEvent {
    pub meta: EventMeta,
    pub id: StoryId,
    pub kind: StoryEventKind,
}

impl StoryEvent {
    pub fn new(meta: EventMeta, id: StoryId, kind: StoryEventKind) -> Self {
        Self { meta, id, kind }
    }
}
