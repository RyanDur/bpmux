pub(in crate::bridge) async fn handle_add_stories_to_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    port: u16,
) -> Result<String, String> {
    crate::bridge::tools_stories::require_role(agent_name, &["pm"], "add stories to iteration")?;
    let story_ids: Vec<u64> = args
        .get("story_ids")
        .and_then(|v| v.as_array())
        .ok_or("missing 'story_ids' argument (array of story numbers)")?
        .iter()
        .filter_map(|v| v.as_u64())
        .collect();
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_number' argument (issue# of the Iteration story)")?;
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = format!("http://127.0.0.1:{port}/api/add_stories_to_iteration");
    let body = serde_json::json!({
        "story_ids": story_ids,
        "iteration_number": iteration_number,
    });
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("add_stories_to_iteration: bpmux-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("add_stories_to_iteration: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("add_stories_to_iteration: {status} — {text}"));
    }
    Ok(text)
}

pub(in crate::bridge) async fn handle_remove_stories_from_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    port: u16,
) -> Result<String, String> {
    crate::bridge::tools_stories::require_role(
        agent_name,
        &["pm"],
        "remove stories from iteration",
    )?;
    let story_ids: Vec<u64> = args
        .get("story_ids")
        .and_then(|v| v.as_array())
        .ok_or("missing 'story_ids' argument (array of story numbers)")?
        .iter()
        .filter_map(|v| v.as_u64())
        .collect();
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = format!("http://127.0.0.1:{port}/api/remove_stories_from_iteration");
    let body = serde_json::json!({ "story_ids": story_ids });
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            format!("remove_stories_from_iteration: bpmux-server unreachable at {url}: {e}")
        })?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("remove_stories_from_iteration: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("remove_stories_from_iteration: {status} — {text}"));
    }
    Ok(text)
}
