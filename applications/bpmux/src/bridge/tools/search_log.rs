use super::helpers::urlencoding;

pub(in crate::bridge) async fn handle_search_log(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("missing 'query' argument")?;
    let mut url = crate::transport::local_url::api_url(
        port,
        &format!("/api/search?query={}", urlencoding(query)),
    );
    if let Some(sender) = args.get("sender").and_then(|v| v.as_str()) {
        url.push_str(&format!("&sender={}", urlencoding(sender)));
    }
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to search: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse response: {e}: {body}"))?;
    let results = parsed["messages"]
        .as_array()
        .ok_or_else(|| format!("unexpected response shape: {body}"))?;
    if results.is_empty() {
        return Ok(format!("No results for \"{query}\"."));
    }
    let mut text = format!("── Search: \"{query}\" ({} results) ──\n", results.len());
    for (i, result) in results.iter().enumerate() {
        text.push_str(&format!("\n[{}]\n", i + 1));
        if let Some(msgs) = result["messages"].as_array() {
            for msg in msgs {
                let from = msg["from"].as_str().unwrap_or("?");
                let sender = from.split(':').next().unwrap_or(from);
                let msg_text = msg["text"].as_str().unwrap_or("");
                text.push_str(&format!("  {sender}: {msg_text}\n"));
            }
        }
    }
    text.push_str("── End of results ──");
    Ok(text)
}
