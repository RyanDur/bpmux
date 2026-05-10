pub(in crate::bridge) async fn handle_create_iteration_marker(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = format!("http://127.0.0.1:{port}/api/iteration_markers");
    let body = serde_json::json!({
        "theme_name": args.get("theme_name").and_then(|v| v.as_str()),
    });
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("create_iteration_marker: bpmux-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("create_iteration_marker: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("create_iteration_marker: {status} — {text}"));
    }
    Ok(text)
}

pub(in crate::bridge) async fn handle_reorder_iteration_marker(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &tokio::sync::Mutex<Option<super::WsSink>>,
) -> Result<String, String> {
    crate::bridge::tools_stories::require_role(agent_name, &["pm"], "reorder iteration marker")?;
    let marker_id = args
        .get("marker_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'marker_id' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as usize;
    let path = crate::session::priority::default_priority_path()
        .map_err(|e| format!("priority path: {e}"))?;
    apply_reorder_iteration_marker(&path, marker_id, position)?;
    crate::bridge::tools_stories::broadcast_board_invalidate(ws_tx).await;
    Ok(format!("marker #{marker_id} moved to position {position}"))
}

pub(in crate::bridge) fn apply_reorder_iteration_marker(
    path: &std::path::Path,
    marker_id: u64,
    position: usize,
) -> Result<(), String> {
    let mut index = crate::session::priority::load_priority(path);
    crate::session::priority::reorder_icebox_unified(
        &mut index,
        crate::session::priority::IceboxItem::Marker { id: marker_id },
        position,
    );
    crate::session::priority::save_priority(path, &index)
        .map_err(|e| format!("save priority: {e}"))?;
    Ok(())
}

pub(in crate::bridge) async fn handle_list_iteration_markers(port: u16) -> Result<String, String> {
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = format!("http://127.0.0.1:{port}/api/iteration_markers");
    let res = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("list_iteration_markers: bpmux-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("list_iteration_markers: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("list_iteration_markers: {status} — {body}"));
    }
    Ok(body)
}
