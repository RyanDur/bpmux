use crate::product_management::board::label_codec::LabelCodec;
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
    let codec = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
    let current_iteration = story.raw_labels.iter().find_map(|l| {
        crate::product_management::iteration::label::parse_iteration_label(&codec, l)
            .map(|p| (p.name, l.clone()))
    });
    let name = current_iteration
        .as_ref()
        .map(|(t, _)| t.clone())
        .unwrap_or_else(|| "Unscheduled".to_string());
    let other_positions: Vec<u32> = refresh
        .all_stories()
        .iter()
        .filter(|s| s.id != target_id && !s.is_closed)
        .filter_map(|s| {
            s.raw_labels.iter().find_map(|l| {
                crate::product_management::iteration::label::parse_iteration_label(&codec, l)
                    .filter(|p| p.name == name)
                    .map(|p| p.position)
            })
        })
        .collect();
    let insert_index = position.saturating_sub(1).min(other_positions.len());
    let new_position = crate::product_management::iteration::label::compute_insert_position(
        insert_index,
        &other_positions,
    );
    let new_label = codec.encode_iteration(&name, new_position);
    if let Some((_, current_label)) = &current_iteration
        && current_label.as_str() == new_label.as_str()
    {
        return Ok(());
    }
    let meta = || crate::product_management::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    if let Some((_, current_label)) = current_iteration {
        refresh
            .mutate_sync(crate::product_management::board::BoardEvent::TagRemoved {
                meta: meta(),
                id: target_id,
                label: current_label,
            })
            .map_err(|e| format!("TagRemoved #{id}: {e}"))?;
    }
    refresh
        .mutate_sync(crate::product_management::board::BoardEvent::TagAdded {
            meta: meta(),
            id: target_id,
            label: new_label,
        })
        .map_err(|e| format!("TagAdded #{id}: {e}"))?;
    Ok(())
}
