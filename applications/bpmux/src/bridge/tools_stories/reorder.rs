use tokio::sync::Mutex;

use super::super::tools::WsSink;
use super::helpers::role_from_name;
use super::signals::broadcast_board_invalidate;
use crate::board::label_codec::LabelCodec;

pub(crate) async fn handle_reorder_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition =
        crate::workflow::resolve_transition(workflow, "reorder", crate::workflow::GateClass::None)
            .map_err(|e| e.as_json_string())?;
    crate::workflow::check_role_allowed(transition, actor_role).map_err(|e| e.as_json_string())?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as usize;
    let state_str = args
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or("missing 'state' argument (e.g., 'ready', 'in_progress', 'delivered')")?;
    let state = crate::board::domain::StoryState::parse(state_str).ok_or_else(|| {
        format!("reorder_story state='{state_str}' not a valid StoryState (use icebox/ready/in-progress/desk-check/delivered/accepted)")
    })?;
    match state {
        crate::board::domain::StoryState::Icebox => {
            let r = refresh.ok_or(
                "reorder_story state=icebox requires refresh-driver (canonical-via-GH label substrate per #1301)",
            )?;
            reorder_icebox_story_via_labels(r, number, position)?;
            broadcast_board_invalidate(ws_tx).await;
            Ok(format!("#{number} moved to position {position} in icebox"))
        }
        crate::board::domain::StoryState::Ready => {
            let r = refresh.ok_or(
                "reorder_story state=ready requires refresh-driver (canonical-via-GH label substrate per #1301)",
            )?;
            reorder_ready_story_via_labels(r, number, position)?;
            broadcast_board_invalidate(ws_tx).await;
            Ok(format!("#{number} moved to position {position} in ready"))
        }
        _ => Err(format!(
            "reorder_story state='{}' not supported (#1301 retired per-state ordering — only 'ready' and 'icebox' are user-orderable)",
            state.as_str()
        )),
    }
}

pub(crate) fn reorder_icebox_story_via_labels(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
    position: usize,
) -> Result<(), String> {
    let story = refresh
        .story(number)
        .ok_or_else(|| format!("story #{number} not found in canonical store"))?;
    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let current_iteration = story.raw_labels.iter().find_map(|l| {
        crate::board::iteration_label::parse_iteration_label(&codec, l)
            .map(|p| (p.theme_name, l.clone()))
    });
    let theme_name = current_iteration
        .as_ref()
        .map(|(t, _)| t.clone())
        .unwrap_or_else(|| "Unscheduled".to_string());
    let other_positions: Vec<u32> = refresh
        .all_stories()
        .iter()
        .filter(|s| s.number != number && !s.is_closed)
        .filter_map(|s| {
            s.raw_labels.iter().find_map(|l| {
                crate::board::iteration_label::parse_iteration_label(&codec, l)
                    .filter(|p| p.theme_name == theme_name)
                    .map(|p| p.position)
            })
        })
        .collect();
    let insert_index = position.saturating_sub(1).min(other_positions.len());
    let new_position =
        crate::board::iteration_label::compute_insert_position(insert_index, &other_positions);
    let new_label = codec.encode_iteration(&theme_name, new_position);
    if let Some((_, current_label)) = &current_iteration
        && current_label.as_str() == new_label.as_str()
    {
        return Ok(());
    }
    let meta = || crate::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    if let Some((_, current_label)) = current_iteration {
        refresh
            .mutate_sync(crate::board::BoardEvent::TagRemoved {
                meta: meta(),
                number,
                label: current_label,
            })
            .map_err(|e| format!("TagRemoved #{number}: {e}"))?;
    }
    refresh
        .mutate_sync(crate::board::BoardEvent::TagAdded {
            meta: meta(),
            number,
            label: new_label,
        })
        .map_err(|e| format!("TagAdded #{number}: {e}"))?;
    Ok(())
}

pub(crate) fn reorder_ready_story_via_labels(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
    position: usize,
) -> Result<(), String> {
    let story = refresh
        .story(number)
        .ok_or_else(|| format!("story #{number} not found in canonical store"))?;
    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let current = story.raw_labels.iter().find_map(|l| {
        crate::board::ready_order_label::parse_ready_order_label(&codec, l).map(|p| (p, l.clone()))
    });
    let other_positions: Vec<u32> = refresh
        .all_stories()
        .iter()
        .filter(|s| {
            s.number != number && !s.is_closed && s.state == crate::board::domain::StoryState::Ready
        })
        .filter_map(|s| {
            s.raw_labels
                .iter()
                .find_map(|l| crate::board::ready_order_label::parse_ready_order_label(&codec, l))
        })
        .collect();
    let insert_index = position.saturating_sub(1).min(other_positions.len());
    let new_position =
        crate::board::iteration_label::compute_insert_position(insert_index, &other_positions);
    let new_label = codec.encode_ready_order(new_position);
    if let Some((_, current_label)) = &current
        && current_label.as_str() == new_label.as_str()
    {
        return Ok(());
    }
    let meta = || crate::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    if let Some((_, current_label)) = current {
        refresh
            .mutate_sync(crate::board::BoardEvent::TagRemoved {
                meta: meta(),
                number,
                label: current_label,
            })
            .map_err(|e| format!("TagRemoved #{number}: {e}"))?;
    }
    refresh
        .mutate_sync(crate::board::BoardEvent::TagAdded {
            meta: meta(),
            number,
            label: new_label,
        })
        .map_err(|e| format!("TagAdded #{number}: {e}"))?;
    Ok(())
}
