use super::helpers::{ensure_story_in_canonical_store, require_purpose};
use crate::product_management::board::RefreshDriver;
use crate::product_management::story::StoryType;
use std::sync::Arc;

pub(crate) async fn handle_edit_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "edit an outcome")?;
    let outcome_number = args
        .get("outcome_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_number' argument")?;
    let title = args.get("title").and_then(serde_json::Value::as_str);
    let body = args.get("body").and_then(serde_json::Value::as_str);
    if title.is_none() && body.is_none() {
        return Err("outcome edit needs a change: pass --title <T> and/or --body <B>".to_string());
    }

    let r = refresh.ok_or("edit_outcome requires refresh (RefreshDriver)")?;
    let outcome = ensure_story_in_canonical_store(r, outcome_number).await?;
    let story_type = outcome.story_type();
    if story_type != StoryType::Outcome {
        return Err(format!(
            "#{outcome_number} is not an Outcome (story_type={:?}); it may be a story, not an outcome — outcomes: bpmux roadmap",
            story_type
        ));
    }

    let mut updated: Vec<&str> = Vec::new();
    if let Some(t) = title {
        r.update_title(outcome_number, t.to_string())
            .await
            .map_err(|e| format!("#{outcome_number} title update failed: {e}"))?;
        updated.push("title");
    }
    if let Some(b) = body {
        r.update_body(outcome_number, b.to_string())
            .await
            .map_err(|e| format!("#{outcome_number} why update failed: {e}"))?;
        updated.push("why");
    }
    Ok(format!("#{outcome_number} {} updated", updated.join(" + ")))
}
