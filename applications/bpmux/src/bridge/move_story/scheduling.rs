use std::sync::Arc;

use crate::sync::SyncOrchestrator;
use product_management::iteration::IterationId;
use product_management::iteration::label::current_iteration_id;
use product_management::story::StoryId;

use super::target::TargetState;

pub(super) fn resolve_scheduling_target(
    select: &crate::store::Select,
    id: u64,
    target: &TargetState,
) -> Result<Option<IterationId>, String> {
    let is_epic = select
        .select(|state| {
            product_management::epic::get(&state.epics, product_management::epic::EpicId::new(id))
        })
        .is_some();
    if is_epic || !matches!(target, TargetState::Ready) {
        return Ok(None);
    }
    let iterations = select.select(|state| product_management::iteration::all(&state.iterations));
    match current_iteration_id(&iterations) {
        Some(iteration_id) => Ok(Some(iteration_id)),
        None => Err(format!(
            "#{id} cannot move to ready: no current iteration designated — ready IS the \
             current iteration's queue. create_iteration + designate_current_iteration, \
             then schedule."
        )),
    }
}

pub(super) async fn apply_scheduling(
    driver: &Arc<SyncOrchestrator>,
    select: &crate::store::Select,
    dispatch: &crate::store::Dispatch,
    id: u64,
    target: &TargetState,
    position: Option<usize>,
    scheduling_target: Option<IterationId>,
) -> Result<(), String> {
    if matches!(target, TargetState::Icebox) {
        crate::bridge::tools_stories::iteration_members::remove_member_everywhere(
            driver, select, dispatch, id,
        )
        .await
        .map_err(|e| format!("#{id} unschedule (iteration-member removal) failed: {e}"))?;
    }
    let Some(iteration_id) = scheduling_target else {
        return Ok(());
    };
    let members = select
        .select(|state| product_management::iteration::get(&state.iterations, iteration_id))
        .map(|iteration| iteration.stories)
        .unwrap_or_default();
    let join_position = match position {
        Some(pos) => Some(pos as u32),
        None if !members.contains(&StoryId::new(id)) => Some(members.len() as u32 + 1),
        None => None,
    };
    if let Some(pos) = join_position {
        crate::bridge::tools_stories::iteration_members::set_member_position(
            driver,
            select,
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
