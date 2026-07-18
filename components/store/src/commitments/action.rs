use serde_json::{Map, Value};

#[derive(Clone, Debug)]
pub enum CommitmentsAction {
    Committed {
        user_id: String,
        event_class: String,
        filter: Option<Map<String, Value>>,
    },
    CommitmentsMatched {
        user_ids: Vec<String>,
        event_class: String,
        payload: Value,
    },
}
