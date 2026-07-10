use crate::product_management::story::events::EventMeta;

pub(crate) fn meta_for(agent_name: &str, actor_id: Option<&str>) -> EventMeta {
    EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        actor_id: actor_id.map(str::to_string),
        ts: chrono::Utc::now(),
    }
}
