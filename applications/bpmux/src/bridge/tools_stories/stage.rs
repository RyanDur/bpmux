use super::helpers::{ensure_story_in_canonical_store, require_nonempty_string, role_from_name};
use crate::product_management::story::events::{EventMeta, StoryEvent};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{BoardState, Story, StoryId, Verdict};
use crate::sync::RefreshDriver;
use crate::workflow::StageId;

const REASON_CITE_MIN_CHARS: usize = 16;

pub(crate) async fn handle_pass_stage(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let (id, r, story, held) = judged_stage_of(args, refresh).await?;
    enforce_judge(&held, agent_name)?;
    let observation = require_nonempty_string(
        args,
        "verified_observation",
        "verified_observation is required: articulate what you verified at this stage in the running app — it becomes the next stage's diff-point",
    )?;
    let workflow = crate::workflow::current_workflow();
    let verdict = Verdict::Succeed {
        step: held.as_str().into(),
    };
    let crate::board::Decision {
        coarse: new_state,
        placement,
    } = crate::board::advance(&verdict, workflow)
        .ok_or_else(|| format!("#{id} pass_stage has no forward state from '{}'", held))?;
    cast_and_transition(
        r,
        id,
        agent_name,
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
    refresh: Option<&std::sync::Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let (id, r, story, held) = judged_stage_of(args, refresh).await?;
    enforce_judge(&held, agent_name)?;
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
    let workflow = crate::workflow::current_workflow();
    let verdict = Verdict::Fail {
        step: held.as_str().into(),
    };
    let crate::board::Decision {
        coarse: new_state,
        placement,
    } = crate::board::advance(&verdict, workflow)
        .ok_or_else(|| format!("#{id} fail_stage has no fail target from '{}'", held))?;
    cast_and_transition(
        r,
        id,
        agent_name,
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
    refresh: Option<&'r std::sync::Arc<dyn RefreshDriver>>,
) -> Result<(u64, &'r std::sync::Arc<dyn RefreshDriver>, Story, StageId), String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let r = refresh.ok_or("stage verbs require refresh (RefreshDriver)")?;
    let story = ensure_story_in_canonical_store(r, id).await?;
    if !matches!(story.state, BoardState::InProgress) {
        return Err(format!(
            "#{id} is not in the board's middle (state={}); stage judgments apply only between pickup and acceptance",
            story.state.as_str()
        ));
    }
    let workflow = crate::workflow::current_workflow();
    let held = r.story_held_stage(&story).or_else(|| {
        crate::workflow::board_middle_stage_ids(workflow)
            .first()
            .cloned()
    });
    let held = held.ok_or_else(|| format!("#{id} the configured workflow has no middle stages"))?;
    Ok((id, r, story, held))
}

fn enforce_judge(held: &StageId, agent_name: &str) -> Result<(), String> {
    let workflow = crate::workflow::current_workflow();
    crate::workflow::enforce_stage_judge(
        workflow,
        held.as_str(),
        role_from_name(agent_name),
        agent_name,
    )
    .map_err(|e| e.as_json_string())
}

fn cast_and_transition(
    r: &std::sync::Arc<dyn RefreshDriver>,
    id: u64,
    agent_name: &str,
    verdict: Verdict,
    payload: Option<String>,
    new_state: BoardState,
    placement: Option<StageId>,
) -> Result<(), String> {
    let meta = EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    };
    r.mutate_batch_sync(vec![
        StoryEvent::VerdictCast {
            meta: meta.clone(),
            id: StoryId::new(id),
            verdict,
            payload,
        },
        StoryEvent::StateTransitioned {
            meta,
            id: StoryId::new(id),
            new_state,
            placement,
        },
    ])
    .map_err(|e| format!("#{id} stage judgment enqueue failed: {e}"))
}
