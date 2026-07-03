use crate::product_management::board::PeerId;
use crate::product_management::board::events::EventMeta;

pub(super) fn meta_for(agent_name: &str) -> EventMeta {
    EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    }
}
