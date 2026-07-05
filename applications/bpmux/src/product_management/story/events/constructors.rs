use super::EventMeta;
use crate::product_management::story::{CastVerdict, FreeComment, Verdict};

impl CastVerdict {
    pub fn from_event(verdict: Verdict, meta: &EventMeta, payload: Option<String>) -> Self {
        Self {
            verdict,
            ts: meta.ts,
            actor: meta.actor.clone(),
            payload,
        }
    }
}

impl FreeComment {
    pub fn from_event(body: String, meta: &EventMeta) -> Self {
        Self {
            body,
            author: meta.actor.clone(),
            ts: meta.ts,
        }
    }
}
