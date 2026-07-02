use crate::product_management::story::StoryId;

pub(crate) fn reorder_story_retired_error() -> String {
    serde_json::json!({
        "kind": "tool-retired",
        "retired_axis": "reorder-tool",
        "successor": "move_story",
        "message": "reorder_story is retired per #1403. Use move_story(id, target_state=<same column>, position=<N>, rationale=<why>) — position-axis is uniform with state-transition substrate via #1392 universal-move. Internal position-mutation algorithm preserved; only the dedicated reorder_story tool retires."
    })
    .to_string()
}

pub(crate) async fn handle_reorder_story(
    _args: &serde_json::Map<String, serde_json::Value>,
    _agent_name: &str,
    _refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    Err(reorder_story_retired_error())
}

pub(crate) fn reorder_icebox_story_via_labels(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    id: u64,
    position: usize,
) -> Result<(), String> {
    let story = refresh
        .story(id)
        .ok_or_else(|| format!("story #{id} not found in canonical store"))?;
    let target_id = StoryId::new(id);
    let current_slot = story.iteration_slot().cloned();
    let name = current_slot
        .as_ref()
        .map(|slot| slot.name.clone())
        .unwrap_or_else(|| "Unscheduled".to_string());
    let other_positions: Vec<u32> = refresh
        .all_stories()
        .iter()
        .filter(|s| s.id != target_id && !s.is_closed)
        .filter_map(|s| {
            s.iteration_slot()
                .filter(|slot| slot.name == name)
                .map(|slot| slot.position)
        })
        .collect();
    let insert_index = position.saturating_sub(1).min(other_positions.len());
    let new_position = crate::product_management::iteration::label::compute_insert_position(
        insert_index,
        &other_positions,
    );
    if current_slot
        .as_ref()
        .is_some_and(|slot| slot.name == name && slot.position == new_position)
    {
        return Ok(());
    }
    refresh
        .mutate_sync(
            crate::product_management::board::StoryEvent::IterationSlotSet {
                meta: crate::product_management::board::events::EventMeta {
                    actor: "pm".to_string(),
                    ts: chrono::Utc::now(),
                    peer_id: crate::product_management::board::PeerId::local(),
                },
                id: target_id,
                name,
                position: new_position,
            },
        )
        .map_err(|e| format!("IterationSlotSet #{id}: {e}"))?;
    Ok(())
}
