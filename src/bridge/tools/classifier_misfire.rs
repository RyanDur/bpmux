use tokio::sync::Mutex;

/// Classifier misfire feedback handler (#159).
///
/// Agents call this tool when they observe a classifier nudge firing on a
/// message that isn't actually a learning moment. POSTs to bpmux's
/// `/api/classifier-misfire` which writes a `MISFIRE` line to the diag
/// log (gated on `BPMUX_LOG` being set). No effect on the classifier
/// itself — this is purely a training-signal capture path so agent
/// knowledge about wrong fires gets externalized instead of silently
/// dismissed.
///
/// The `reason` argument is OPTIONAL per the acceptance criteria: agents
/// can mark a fire as wrong without having to explain why. Per Naur's
/// "state observation, not conclusion" framing.
pub(in crate::bridge) async fn handle_classifier_misfire(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
    name: &Mutex<String>,
) -> Result<String, String> {
    let nudge_hash = args
        .get("nudge_hash")
        .and_then(|v| v.as_str())
        .ok_or("missing 'nudge_hash' argument")?
        .to_string();
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .map(String::from);
    let agent = name.lock().await.clone();
    let body = serde_json::json!({
        "agent": agent,
        "nudge_hash": nudge_hash,
        "reason": reason,
    });
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/classifier-misfire");
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed to record misfire: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("misfire failed ({status}): {body}"));
    }
    Ok("misfire recorded".to_string())
}
