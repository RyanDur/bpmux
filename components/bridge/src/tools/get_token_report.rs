pub async fn handle_get_token_report() -> Result<String, String> {
    use telemetry::tokens::{build_token_report, default_tokens_log_path, read_token_rows};
    let path = default_tokens_log_path().map_err(|e| format!("path resolution failed: {e}"))?;
    let rows = read_token_rows(&path).map_err(|e| format!("log read failed: {e}"))?;
    let report = build_token_report(&rows);
    let view = crate::view_model::TokenReportView::from(&report);
    serde_json::to_string_pretty(&view).map_err(|e| format!("serialization failed: {e}"))
}
