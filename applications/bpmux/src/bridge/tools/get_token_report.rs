/// `get_token_report` (#179) — reads the token log directly from disk
/// and returns a structured summary as JSON. Bypasses the HTTP API
/// because the log is a file on the local filesystem that both the
/// bpmux TUI process and the bridge process can read independently.
/// Returns an empty report (no rows, zero totals) if the log file
/// doesn't exist yet — the first-use case, not an error.
pub(in crate::bridge) async fn handle_get_token_report() -> Result<String, String> {
    use crate::session::tokens::{build_token_report, default_tokens_log_path, read_token_rows};
    let path = default_tokens_log_path().map_err(|e| format!("path resolution failed: {e}"))?;
    let rows = read_token_rows(&path).map_err(|e| format!("log read failed: {e}"))?;
    let report = build_token_report(&rows);
    let view = crate::bridge::view_model::TokenReportView::from(&report);
    serde_json::to_string_pretty(&view).map_err(|e| format!("serialization failed: {e}"))
}
