use crate::message::{Message, MessageType};
use crate::product_management::evidence::{
    AddEvidenceOutcome, Evidence, add_or_replace_evidence, load, save,
};
use crate::product_management::story::StoryId;
use std::sync::Arc;

pub(in crate::bridge) async fn handle_submit_verdict(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let role = agent_name.split(':').next().unwrap_or(agent_name);
    let workflow = crate::workflow::current_workflow();
    let dev_id = workflow.role_id_for_purpose("dev").unwrap_or("dev");
    let designer_id = workflow
        .role_id_for_purpose("designer")
        .unwrap_or("designer");
    if role != dev_id && role != designer_id {
        return Err(format!(
            "submit_verdict is restricted to {dev_id} or {designer_id}; agent '{agent_name}' is role '{role}'"
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

    let ts = chrono::Utc::now();
    let entry = Evidence {
        story_id: StoryId::new(story_id),
        from_role: role.to_string(),
        verdict: verdict_value.clone(),
        substrate_grounding,
        ts,
        grounded_head: crate::bridge::tools::git_repo::current_head_sha(),
    };

    let mut store = load(&session.dir);
    let outcome = add_or_replace_evidence(&mut store, entry);
    save(&session.dir, &store).map_err(|e| format!("save verdicts: {e}"))?;

    let pm_id = workflow.role_id_for_purpose("pm").unwrap_or("pm");
    let notification = Message {
        from: "bpmux".to_string(),
        text: format!("verdict cast on #{story_id} from {role}: {verdict_value}"),
        ts,
        message_type: MessageType::System,
        addressee: Some(pm_id.to_string()),
        ..Message::default()
    };
    if let Err(e) = session.broadcast(&notification) {
        crate::diag::log_diag(&format!(
            "submit_verdict: verdict-cast broadcast failed for #{story_id}: {e}"
        ));
    }

    let outcome_label = match outcome {
        AddEvidenceOutcome::Inserted => "new",
        AddEvidenceOutcome::Replaced => "replaced prior",
    };
    Ok(format!(
        "verdict recorded on #{story_id} from {role} ({outcome_label})"
    ))
}
