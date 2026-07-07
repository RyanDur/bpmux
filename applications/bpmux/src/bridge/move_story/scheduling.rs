use std::sync::Arc;

use crate::product_management::iteration::IterationId;
use crate::product_management::iteration::label::current_iteration_id;
use crate::product_management::story::StoryId;
use crate::sync::RefreshDriver;

use super::target::TargetState;

pub(super) fn resolve_scheduling_target(
    r: &Arc<dyn RefreshDriver>,
    id: u64,
    target: &TargetState,
) -> Result<Option<IterationId>, String> {
    let is_epic = r
        .epic(crate::product_management::epic::EpicId::new(id))
        .is_some();
    if is_epic || !matches!(target, TargetState::Ready) {
        return Ok(None);
    }
    match current_iteration_id(&r.iterations()) {
        Some(iteration_id) => Ok(Some(iteration_id)),
        None => Err(format!(
            "#{id} cannot move to ready: no current iteration designated — ready IS the \
             current iteration's queue. create_iteration + designate_current_iteration, \
             then schedule."
        )),
    }
}

pub(super) async fn apply_scheduling(
    r: &Arc<dyn RefreshDriver>,
    dispatch: &crate::store::Dispatch,
    id: u64,
    target: &TargetState,
    position: Option<usize>,
    scheduling_target: Option<IterationId>,
) -> Result<(), String> {
    if matches!(target, TargetState::Icebox) {
        crate::bridge::tools_stories::iteration_members::remove_member_everywhere(r, dispatch, id)
            .await
            .map_err(|e| format!("#{id} unschedule (iteration-member removal) failed: {e}"))?;
        if let Some(pos) = position {
            crate::bridge::tools_stories::reorder_icebox_story_via_labels(r, id, pos)?;
        }
    }
    let Some(iteration_id) = scheduling_target else {
        return Ok(());
    };
    let members = r
        .iterations()
        .into_iter()
        .find(|it| it.id == iteration_id)
        .map(|it| it.stories)
        .unwrap_or_default();
    let join_position = match position {
        Some(pos) => Some(pos as u32),
        None if !members.contains(&StoryId::new(id)) => Some(members.len() as u32 + 1),
        None => None,
    };
    if let Some(pos) = join_position {
        crate::bridge::tools_stories::iteration_members::set_member_position(
            r,
            dispatch,
            iteration_id.value(),
            id,
            pos,
        )
        .await
        .map_err(|e| format!("#{id} schedule (iteration-member position) failed: {e}"))?;
    }
    Ok(())
}
