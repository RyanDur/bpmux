pub(in crate::bridge) async fn handle_capture_screen(port: u16) -> Result<String, String> {
    let url = crate::transport::local_url::api_url(port, "/api/screen");
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to capture: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read: {e}"))?;
    Ok(body)
}
