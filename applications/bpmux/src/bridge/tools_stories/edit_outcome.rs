use super::helpers::{ensure_outcome_in_canonical_store, require_purpose};
use crate::product_management::board::RefreshDriver;
use std::sync::Arc;

pub(crate) async fn handle_edit_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "edit an outcome")?;
    let outcome_id = args
        .get("outcome_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_id' argument")?;
    let title = args.get("title").and_then(serde_json::Value::as_str);
    let body = args.get("body").and_then(serde_json::Value::as_str);
    if title.is_none() && body.is_none() {
        return Err("outcome edit needs a change: pass --title <T> and/or --body <B>".to_string());
    }

    let r = refresh.ok_or("edit_outcome requires refresh (RefreshDriver)")?;
    ensure_outcome_in_canonical_store(r, outcome_id).await?;

    let mut updated: Vec<&str> = Vec::new();
    if let Some(t) = title {
        r.update_title(outcome_id, t.to_string())
            .await
            .map_err(|e| format!("#{outcome_id} title update failed: {e}"))?;
        updated.push("title");
    }
    if let Some(b) = body {
        r.update_body(outcome_id, b.to_string())
            .await
            .map_err(|e| format!("#{outcome_id} why update failed: {e}"))?;
        updated.push("why");
    }
    Ok(format!("#{outcome_id} {} updated", updated.join(" + ")))
}
