use serde_json::{Map, Value};

#[derive(Clone, Debug)]
pub enum CommitmentsAction {
    Committed {
        role: String,
        event_class: String,
        filter: Option<Map<String, Value>>,
    },
    CommitmentsMatched {
        roles: Vec<String>,
        event_class: String,
        payload: Value,
    },
}
