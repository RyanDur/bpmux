use super::state_delta::StateDeltaPayload;

impl StateDeltaPayload {
    pub fn to_subscription_event(&self, ts: &str) -> Option<(&'static str, serde_json::Value)> {
        match self {
            StateDeltaPayload::BoardInvalidated { reason } => {
                Some(("board_invalidated", serde_json::json!({ "reason": reason })))
            }
            StateDeltaPayload::Ci { snapshot } => Some((
                "ci_state_changed",
                serde_json::json!({
                    "conclusion": serde_json::to_value(snapshot.state)
                        .unwrap_or(serde_json::Value::Null),
                    "run_id": snapshot.run_id,
                    "head_sha": snapshot.head_sha,
                    "display_title": snapshot.display_title,
                    "url": snapshot
                        .url
                        .clone()
                        .unwrap_or_else(|| crate::GITHUB_BASE_URL.to_string()),
                }),
            )),
            StateDeltaPayload::StoryTransition { transition } => Some((
                "story_state_transition",
                serde_json::json!({
                    "story_id": transition.story_id,
                    "from_state": transition.from_state,
                    "target_state": transition.target_state,
                    "actor": transition.actor,
                    "ts": ts,
                }),
            )),
            _ => None,
        }
    }
}

pub fn payload_matches_filter(
    filter: Option<&serde_json::Map<String, serde_json::Value>>,
    payload: &serde_json::Value,
) -> bool {
    match filter {
        None => true,
        Some(criteria) => criteria
            .iter()
            .all(|(key, expected)| payload.get(key) == Some(expected)),
    }
}
