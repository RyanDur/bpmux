use super::helpers::{ensure_outcome_in_canonical_store, require_purpose};
use source_codec::mapper::outcome_with_body;
use std::sync::Arc;
use sync::SyncOrchestrator;

pub async fn handle_edit_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&Arc<SyncOrchestrator>>,
    dispatch: &store::Dispatch,
    select: &store::Select,
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

    let driver = refresh.ok_or("edit_outcome requires refresh")?;
    let mut outcome = ensure_outcome_in_canonical_store(driver, select, outcome_id).await?;

    let mut updated: Vec<&str> = Vec::new();
    if let Some(t) = title {
        driver
            .update_title(outcome_id, t.to_string())
            .map_err(|e| format!("#{outcome_id} title update failed: {e}"))?;
        outcome = outcome.with_title(t);
        updated.push("title");
    }
    if let Some(b) = body {
        driver
            .update_body(outcome_id, b.to_string())
            .map_err(|e| format!("#{outcome_id} why update failed: {e}"))?;
        outcome = outcome_with_body(&outcome, b);
        updated.push("why");
    }
    dispatch.dispatch(store::AppAction::Outcome(
        product_management::outcome::OutcomeAction::Upserted(vec![outcome]),
    ));
    Ok(format!("#{outcome_id} {} updated", updated.join(" + ")))
}
