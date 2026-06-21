use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::{Map, Value};

use crate::session::message::payload_matches_filter;

struct Commitment {
    event_class: String,
    filter: Option<Map<String, Value>>,
}

#[derive(Default)]
pub struct CommitmentStore {
    by_role: Mutex<HashMap<String, Vec<Commitment>>>,
}

impl CommitmentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commit(&self, role: &str, event_class: &str, filter: Option<Map<String, Value>>) {
        self.by_role
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(role.to_string())
            .or_default()
            .push(Commitment {
                event_class: event_class.to_string(),
                filter,
            });
    }

    pub fn take_matching(&self, event_class: &str, payload: &Value) -> Vec<String> {
        let mut woken = Vec::new();
        let mut by_role = self.by_role.lock().unwrap_or_else(|e| e.into_inner());
        by_role.retain(|role, commitments| {
            let mut fired = false;
            commitments.retain(|commitment| {
                let matches = commitment.event_class == event_class
                    && payload_matches_filter(commitment.filter.as_ref(), payload);
                fired |= matches;
                !matches
            });
            if fired {
                woken.push(role.clone());
            }
            !commitments.is_empty()
        });
        woken
    }
}
