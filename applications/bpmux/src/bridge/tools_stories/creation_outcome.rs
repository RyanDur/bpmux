use super::helpers::{require_nonempty_string, role_from_name};
use crate::product_management::story::StoryId;

pub(crate) async fn handle_create_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    let workflow = crate::workflow::current_workflow();
    let pm = workflow.role_id_for_purpose("pm").unwrap_or("pm");
    let actor_role = role_from_name(agent_name);
    let is_owner = workflow
        .owner_role_id()
        .is_some_and(|o| !o.is_empty() && actor_role == o);
    if actor_role != pm && !is_owner {
        return Err(
            "create_outcome is pm-lane. Outcomes are PM-owned roadmap entities (the why), distinct from board epics."
                .to_string(),
        );
    }
    let title = require_nonempty_string(
        args,
        "title",
        "title is required: the measurable behaviour-change the outcome pursues (the why).",
    )?
    .to_string();
    let horizon_raw = require_nonempty_string(
        args,
        "horizon",
        "horizon is required: one of now | next | later (sequence, not calendar).",
    )?;
    let horizon = match horizon_raw.to_ascii_lowercase().as_str() {
        "now" => crate::product_management::story::Horizon::Now,
        "next" => crate::product_management::story::Horizon::Next,
        "later" => crate::product_management::story::Horizon::Later,
        other => {
            return Err(format!(
                "invalid horizon '{other}': must be now, next, or later (sequence, not calendar)"
            ));
        }
    };
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("## {title}\n"));
    let r = refresh
        .ok_or("create_outcome requires refresh (RefreshDriver) — canonical store unavailable")?;
    let id = r
        .create_story(title.clone(), body.clone(), vec!["outcome".to_string()])
        .await
        .map_err(|e| format!("create_outcome adapter call failed: {e}"))?;
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(crate::product_management::board::BoardEvent::TagAdded {
        meta,
        id: StoryId::new(id),
        label: crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .encode_horizon(horizon),
    })
    .map_err(|e| format!("#{id} outcome horizon label enqueue failed: {e}"))?;
    dispatch.dispatch(crate::store::AppAction::Outcome(
        crate::product_management::outcome::OutcomeAction::Upserted(vec![
            crate::product_management::outcome::Outcome {
                id: crate::product_management::outcome::OutcomeId::new(id),
                title: title.clone(),
                body: Some(body),
                filed_at: chrono::Utc::now(),
                filed_by: crate::transport::ws::short_name(agent_name).to_string(),
                lifecycle: crate::product_management::outcome::OutcomeLifecycle::Active {
                    horizon: Some(horizon),
                    horizon_order: None,
                },
                iterations: Vec::new(),
            },
        ]),
    ));
    Ok(format!(
        "outcome created: #{id} {title} (horizon {})",
        horizon.as_str()
    ))
}
