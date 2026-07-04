use crate::product_management::story::events::EventMeta;
use crate::product_management::story::peer::PeerId;

pub(super) fn meta_for(agent_name: &str) -> EventMeta {
    EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    }
}
