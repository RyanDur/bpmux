use std::collections::HashMap;

use serde_json::{Map, Value};

#[derive(Clone)]
pub struct Commitment {
    pub event_class: String,
    pub filter: Option<Map<String, Value>>,
}

#[derive(Default, Clone)]
pub struct CommitmentsState {
    pub by_role: HashMap<String, Vec<Commitment>>,
}
