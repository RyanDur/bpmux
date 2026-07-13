use super::helpers::{ensure_outcome_in_canonical_store, require_purpose};
use crate::integrations::github::mapper::outcome_with_body;
use crate::sync::RefreshDriver;
use std::sync::Arc;

pub(crate) async fn handle_edit_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "edit an outcome")?;
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
    let mut outcome = ensure_outcome_in_canonical_store(r, outcome_id).await?;

    let mut updated: Vec<&str> = Vec::new();
    if let Some(t) = title {
        r.update_title(outcome_id, t.to_string())
            .await
            .map_err(|e| format!("#{outcome_id} title update failed: {e}"))?;
        outcome = outcome.with_title(t);
        updated.push("title");
    }
    if let Some(b) = body {
        r.update_body(outcome_id, b.to_string())
            .await
            .map_err(|e| format!("#{outcome_id} why update failed: {e}"))?;
        outcome = outcome_with_body(&outcome, b);
        updated.push("why");
    }
    dispatch.dispatch(crate::store::AppAction::Outcome(
        product_management::outcome::OutcomeAction::Upserted(vec![outcome]),
    ));
    Ok(format!("#{outcome_id} {} updated", updated.join(" + ")))
}
