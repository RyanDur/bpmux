use crate::product_management::story::events::EventMeta;

pub(super) fn meta_for(agent_name: &str) -> EventMeta {
    EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
    }
}
