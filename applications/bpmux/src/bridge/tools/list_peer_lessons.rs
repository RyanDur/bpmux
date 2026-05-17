use super::helpers::urlencoding;

pub(in crate::bridge) async fn handle_list_peer_lessons(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let mut url = crate::transport::local_url::api_url(port, "/api/peer-lessons");
    if let Some(peer) = args.get("peer").and_then(|v| v.as_str()) {
        url.push_str(&format!("?peer={}", urlencoding(peer)));
    }
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read: {e}"))?;
    Ok(body)
}
