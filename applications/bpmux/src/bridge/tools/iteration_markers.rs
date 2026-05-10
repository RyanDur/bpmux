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

#[derive(Debug, serde::Serialize)]
pub(crate) struct AddStoriesOutcome {
    pub stories: Vec<StoryPosition>,
    pub marker: MarkerPosition,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct StoryPosition {
    pub id: u64,
    pub position: usize,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct MarkerPosition {
    pub id: u64,
    pub position: usize,
    pub theme_name: String,
}

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
    let marker_id = args.get("marker_id").and_then(|v| v.as_u64());
    let theme_name = args.get("theme_name").and_then(|v| v.as_str());
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = format!("http://127.0.0.1:{port}/api/add_stories_to_iteration");
    let body = serde_json::json!({
        "story_ids": story_ids,
        "marker_id": marker_id,
        "theme_name": theme_name,
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

pub(crate) fn apply_add_stories_to_iteration(
    priority_path: &std::path::Path,
    session_dir: &std::path::Path,
    story_ids: &[u64],
    marker_id: Option<u64>,
    theme_name: Option<String>,
) -> Result<AddStoriesOutcome, String> {
    if story_ids.is_empty() {
        return Err("'story_ids' must contain at least one story number".to_string());
    }
    let priority_snapshot = crate::session::priority::load_priority(priority_path);
    let markers_snapshot = crate::session::iteration_markers::load(session_dir);
    let mut priority = priority_snapshot.clone();
    let mut markers_store = markers_snapshot.clone();

    let target_marker_id =
        resolve_or_create_marker(&mut priority, &mut markers_store, marker_id, theme_name)?;
    locate_marker_in_unified(&priority, target_marker_id)?;
    let story_positions =
        insert_stories_above_marker_in_input_order(&mut priority, story_ids, target_marker_id);
    let final_marker_idx = locate_marker_in_unified(&priority, target_marker_id)?;
    let theme = markers_store
        .markers
        .iter()
        .find(|m| m.id == target_marker_id)
        .map(|m| m.theme_name.clone())
        .unwrap_or_default();

    crate::session::priority::save_priority(priority_path, &priority)
        .map_err(|e| format!("save priority: {e}"))?;
    if let Err(e) = crate::session::iteration_markers::save(session_dir, &markers_store) {
        if let Err(rb) = crate::session::priority::save_priority(priority_path, &priority_snapshot)
        {
            return Err(format!(
                "save iteration_markers: {e}; rollback also failed: {rb}"
            ));
        }
        return Err(format!(
            "save iteration_markers: {e} (priority rolled back)"
        ));
    }

    Ok(AddStoriesOutcome {
        stories: story_positions,
        marker: MarkerPosition {
            id: target_marker_id,
            position: final_marker_idx + 1,
            theme_name: theme,
        },
    })
}

fn resolve_or_create_marker(
    priority: &mut crate::session::priority::PriorityIndex,
    markers_store: &mut crate::session::iteration_markers::IterationMarkersStore,
    marker_id: Option<u64>,
    theme_name: Option<String>,
) -> Result<u64, String> {
    if let Some(id) = marker_id {
        return Ok(id);
    }
    let theme = theme_name.ok_or_else(|| "must provide 'marker_id' or 'theme_name'".to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let new_marker = crate::session::iteration_markers::add(markers_store, theme, now);
    priority
        .icebox_unified
        .push(crate::session::priority::IceboxItem::Marker { id: new_marker.id });
    Ok(new_marker.id)
}

fn locate_marker_in_unified(
    priority: &crate::session::priority::PriorityIndex,
    target_marker_id: u64,
) -> Result<usize, String> {
    priority
        .icebox_unified
        .iter()
        .position(|item| {
            matches!(
                item,
                crate::session::priority::IceboxItem::Marker { id } if *id == target_marker_id
            )
        })
        .ok_or_else(|| format!("marker_id {target_marker_id} not in icebox_unified"))
}

fn insert_stories_above_marker_in_input_order(
    priority: &mut crate::session::priority::PriorityIndex,
    story_ids: &[u64],
    target_marker_id: u64,
) -> Vec<StoryPosition> {
    let mut positions = Vec::with_capacity(story_ids.len());
    for &story_id in story_ids {
        let item = crate::session::priority::IceboxItem::Story { number: story_id };
        priority.icebox_unified.retain(|x| x != &item);
        if let Some(col) = priority.columns.get_mut("icebox") {
            col.retain(|&n| n != story_id);
        }
        let marker_idx = priority
            .icebox_unified
            .iter()
            .position(|item| {
                matches!(
                    item,
                    crate::session::priority::IceboxItem::Marker { id } if *id == target_marker_id
                )
            })
            .unwrap_or(priority.icebox_unified.len());
        priority.icebox_unified.insert(marker_idx, item);
        positions.push(StoryPosition {
            id: story_id,
            position: marker_idx + 1,
        });
    }
    positions
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
