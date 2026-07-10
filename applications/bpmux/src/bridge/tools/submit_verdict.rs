use crate::product_management::evidence::{EvidenceAction, evidence_for_story};
use crate::product_management::story::StoryId;
use message::{Message, MessageType};
use std::sync::Arc;

pub(in crate::bridge) async fn handle_submit_verdict(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    hub: &Arc<crate::hub::Hub>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    let workflow = crate::workflow::current_workflow();
    let dev_name = workflow.role_name_for_purpose("dev").unwrap_or("dev");
    let designer_name = workflow
        .role_name_for_purpose("designer")
        .unwrap_or("designer");
    if role != dev_name && role != designer_name {
        return Err(format!(
            "submit_verdict is restricted to {dev_name} or {designer_name}; agent '{agent_name}' is role '{role}'"
        ));
    }
    let story_id = args
        .get("story_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'story_id' argument")?;
    let verdict_value = args
        .get("verdict")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("missing 'verdict' argument (non-empty string)")?
        .to_string();
    let substrate_grounding = args
        .get("substrate_grounding")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("missing 'substrate_grounding' argument (non-empty string)")?
        .to_string();

    let replaces_prior = select.select(|state| {
        evidence_for_story(&state.evidence, StoryId::new(story_id))
            .iter()
            .any(|v| v.from_role == role)
    });

    let ts = chrono::Utc::now();
    dispatch.dispatch(crate::store::AppAction::Evidence(
        EvidenceAction::VerdictSubmitted {
            story_id: StoryId::new(story_id),
            from_role: role.to_string(),
            verdict: verdict_value.clone(),
            substrate_grounding,
            ts,
            grounded_head: crate::bridge::tools::git_repo::current_head_sha(),
        },
    ));

    let pm_name = workflow.role_name_for_purpose("pm").unwrap_or("pm");
    let notification = Message {
        from: "bpmux".to_string(),
        text: format!("verdict cast on #{story_id} from {role}: {verdict_value}"),
        ts,
        message_type: MessageType::System,
        addressee: Some(pm_name.to_string()),
        ..Message::default()
    };
    if let Err(e) = hub.broadcast(&notification) {
        diag::log_diag(&format!(
            "submit_verdict: verdict-cast broadcast failed for #{story_id}: {e}"
        ));
    }

    let outcome_label = if replaces_prior {
        "replaced prior"
    } else {
        "new"
    };
    Ok(format!(
        "verdict recorded on #{story_id} from {role} ({outcome_label})"
    ))
}
