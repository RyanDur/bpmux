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
            p @ StateDeltaPayload::StoryUpserted { .. } => named("story_upserted", p),
            p @ StateDeltaPayload::StoryRemoved { .. } => named("story_removed", p),
            p @ StateDeltaPayload::StoryOrder { .. } => named("story_order", p),
            p @ StateDeltaPayload::EpicUpserted { .. } => named("epic_upserted", p),
            p @ StateDeltaPayload::EpicRemoved { .. } => named("epic_removed", p),
            p @ StateDeltaPayload::EpicOrder { .. } => named("epic_order", p),
            p @ StateDeltaPayload::OutcomeUpserted { .. } => named("outcome_upserted", p),
            p @ StateDeltaPayload::OutcomeRemoved { .. } => named("outcome_removed", p),
            p @ StateDeltaPayload::OutcomeOrder { .. } => named("outcome_order", p),
            p @ StateDeltaPayload::IterationUpserted { .. } => named("iteration_upserted", p),
            p @ StateDeltaPayload::IterationRemoved { .. } => named("iteration_removed", p),
            p @ StateDeltaPayload::IterationOrder { .. } => named("iteration_order", p),
            p @ StateDeltaPayload::RoadmapCardUpserted { .. } => named("roadmap_card_upserted", p),
            p @ StateDeltaPayload::RoadmapCardRemoved { .. } => named("roadmap_card_removed", p),
            p @ StateDeltaPayload::RoadmapOrder { .. } => named("roadmap_order", p),
            p @ StateDeltaPayload::ReactionUpdated { .. } => named("reaction_updated", p),
            p @ StateDeltaPayload::RosterMemberUpserted { .. } => {
                named("roster_member_upserted", p)
            }
            p @ StateDeltaPayload::RosterMemberRemoved { .. } => named("roster_member_removed", p),
            p @ StateDeltaPayload::IterationClose { .. } => named("iteration_close", p),
            p @ StateDeltaPayload::IterationCloseBlocked { .. } => {
                named("iteration_close_blocked", p)
            }
            _ => None,
        }
    }
}

fn named(
    event_class: &'static str,
    payload: &StateDeltaPayload,
) -> Option<(&'static str, serde_json::Value)> {
    let mut body = serde_json::to_value(payload).ok()?;
    body.as_object_mut()?.remove("domain");
    Some((event_class, body))
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
