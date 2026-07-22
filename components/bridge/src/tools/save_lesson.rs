use tokio::sync::Mutex;

pub async fn handle_save_lesson(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
    name: &Mutex<String>,
) -> Result<String, String> {
    let topic = args
        .get("topic")
        .and_then(|v| v.as_str())
        .ok_or("missing 'topic' argument")?;
    let lesson = args
        .get("lesson")
        .and_then(|v| v.as_str())
        .ok_or("missing 'lesson' argument")?;
    let evidence: Vec<String> = args
        .get("evidence")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let role = name.lock().await.clone();
    let body = serde_json::json!({
        "topic": topic,
        "lesson": lesson,
        "author_role": role,
        "evidence": evidence,
    });
    let client = reqwest::Client::new();
    let url = local_url::api_url(port, "/api/lessons");
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed to save: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("save failed ({status}): {body}"));
    }
    Ok(format!("lesson saved: {topic}"))
}
