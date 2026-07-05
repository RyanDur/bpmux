use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::product_management::story::peer::PeerId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMeta {
    pub actor: String,
    pub ts: DateTime<Utc>,
    pub peer_id: PeerId,
}

pub fn resolve_order_key(meta: &EventMeta) -> (DateTime<Utc>, String) {
    (meta.ts, meta.peer_id.as_str().to_string())
}

pub fn event_wins(a: &EventMeta, b: &EventMeta) -> bool {
    resolve_order_key(a) > resolve_order_key(b)
}
