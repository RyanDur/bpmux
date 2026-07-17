use super::event_meta::meta_for;
use super::helpers::{ensure_story_in_canonical_store, require_nonempty_string};
use crate::sync::SyncOrchestrator;
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{Story, StoryId, Verdict, Work};
use workflow::StageId;

const REASON_CITE_MIN_CHARS: usize = 16;

pub(crate) async fn handle_pass_stage(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<SyncOrchestrator>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    let (id, driver, story, held) = judged_stage_of(args, refresh, select).await?;
    enforce_judge(&held, role, agent_name)?;
    let observation = require_nonempty_string(
        args,
        "verified_observation",
        "verified_observation is required: articulate what you verified at this stage in the running app — it becomes the next stage's diff-point",
    )?;
    let workflow = workflow::current_workflow();
    let verdict = Verdict::Succeed {
        step: held.as_str().into(),
    };
    let board::Decision {
        coarse: new_state,
        placement,
    } = board::advance(&verdict, workflow)
        .ok_or_else(|| format!("#{id} pass_stage has no forward state from '{}'", held))?;
    cast_and_transition(
        driver,
        id,
        meta_for(agent_name, actor_id),
        verdict,
        Some(observation.to_string()),
        new_state,
        placement,
    )?;
    let _ = story;
    Ok(format!("#{id} passed '{held}': signal sent, board updated"))
}

pub(crate) async fn handle_fail_stage(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<SyncOrchestrator>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    let (id, driver, story, held) = judged_stage_of(args, refresh, select).await?;
    enforce_judge(&held, role, agent_name)?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    let trimmed = reason.trim();
    if trimmed.len() < REASON_CITE_MIN_CHARS {
        let err = serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "fail_stage",
            "missing_evidence": ["falsifier-cite"],
            "min_chars": REASON_CITE_MIN_CHARS,
            "observed_chars": trimmed.len(),
        });
        return Err(err.to_string());
    }
    let workflow = workflow::current_workflow();
    let verdict = Verdict::Fail {
        step: held.as_str().into(),
    };
    let board::Decision {
        coarse: new_state,
        placement,
    } = board::advance(&verdict, workflow)
        .ok_or_else(|| format!("#{id} fail_stage has no fail target from '{}'", held))?;
    cast_and_transition(
        driver,
        id,
        meta_for(agent_name, actor_id),
        verdict,
        Some(reason.to_string()),
        new_state,
        placement,
    )?;
    let _ = story;
    Ok(format!("#{id} failed '{held}': signal sent, board updated"))
}

async fn judged_stage_of<'r>(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&'r std::sync::Arc<SyncOrchestrator>>,
    select: &crate::store::Select,
) -> Result<(u64, &'r std::sync::Arc<SyncOrchestrator>, Story, StageId), String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let driver = refresh.ok_or("stage verbs require refresh (RefreshDriver)")?;
    let story = ensure_story_in_canonical_store(driver, select, id).await?;
    if !matches!(story.state, Work::InProgress) {
        return Err(format!(
            "#{id} is not in the board's middle (state={}); stage judgments apply only between pickup and acceptance",
            story.state.as_str()
        ));
    }
    let workflow = workflow::current_workflow();
    let held = select
        .select(|state| board::held_stage(&story, &state.board))
        .or_else(|| workflow::board_middle_stage_ids(workflow).first().cloned());
    let held = held.ok_or_else(|| format!("#{id} the configured workflow has no middle stages"))?;
    Ok((id, driver, story, held))
}

fn enforce_judge(held: &StageId, role: &str, agent_name: &str) -> Result<(), String> {
    let workflow = workflow::current_workflow();
    workflow::enforce_stage_judge(workflow, held.as_str(), role, agent_name)
        .map_err(|e| e.as_json_string())
}

fn cast_and_transition(
    driver: &std::sync::Arc<SyncOrchestrator>,
    id: u64,
    meta: product_management::story::events::EventMeta,
    verdict: Verdict,
    payload: Option<String>,
    new_state: Work,
    placement: Option<StageId>,
) -> Result<(), String> {
    driver
        .mutate_batch(vec![
            StoryEvent::new(
                meta.clone(),
                StoryId::new(id),
                StoryEventKind::VerdictCast { verdict, payload },
            ),
            StoryEvent::new(
                meta,
                StoryId::new(id),
                StoryEventKind::StateTransitioned {
                    new_state,
                    placement,
                },
            ),
        ])
        .map_err(|e| format!("#{id} stage judgment enqueue failed: {e}"))
}
