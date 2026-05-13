/// #1288: pure helper to recompute + persist marker.stories fold-
/// snapshots after a write-path mutates icebox_unified. Used at every
/// add/remove/reorder site so the snapshot stays consistent with the
/// canonical positional substrate. Save-failures are propagated so
/// callers can roll back priority on a partial-write.
fn refresh_marker_fold_snapshots(
    session_dir: &std::path::Path,
    icebox_unified: &[crate::session::priority::IceboxItem],
) -> Result<(), String> {
    let mut store = crate::session::iteration_markers::load(session_dir);
    crate::session::iteration_markers::sync_marker_fold_snapshots(&mut store, icebox_unified);
    crate::session::iteration_markers::save(session_dir, &store)
        .map_err(|e| format!("save iteration_markers (fold-snapshot): {e}"))
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct StoryPosition {
    pub id: u64,
    pub position: usize,
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

#[derive(Debug, serde::Serialize)]
pub(crate) struct RemoveStoriesOutcome {
    pub stories: Vec<StoryPosition>,
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

pub(crate) fn apply_remove_stories_from_iteration(
    priority_path: &std::path::Path,
    session_dir: &std::path::Path,
    story_ids: &[u64],
) -> Result<RemoveStoriesOutcome, String> {
    if story_ids.is_empty() {
        return Err("'story_ids' must contain at least one story number".to_string());
    }
    let priority_snapshot = crate::session::priority::load_priority(priority_path);
    let mut priority = priority_snapshot.clone();
    let story_positions = relocate_stories_below_bottom_marker(&mut priority, story_ids);

    crate::session::priority::save_priority(priority_path, &priority).map_err(|save_err| {
        if let Err(rb) = crate::session::priority::save_priority(priority_path, &priority_snapshot)
        {
            format!("save priority: {save_err}; rollback also failed: {rb}")
        } else {
            format!("save priority: {save_err} (rolled back to snapshot)")
        }
    })?;

    // #1288: refresh fold-snapshots so the just-removed stories are
    // no longer attributed to any marker. Save-failure here is logged
    // via diag but doesn't roll back priority — fold-snapshot is
    // recoverable from the next mutation OR boot-heal.
    if let Err(e) = refresh_marker_fold_snapshots(session_dir, &priority.icebox_unified) {
        crate::session::diag::log_diag(&format!(
            "[#1288] apply_remove_stories_from_iteration: fold-snapshot refresh failed: {e}"
        ));
    }

    Ok(RemoveStoriesOutcome {
        stories: story_positions,
    })
}

fn relocate_stories_below_bottom_marker(
    priority: &mut crate::session::priority::PriorityIndex,
    story_ids: &[u64],
) -> Vec<StoryPosition> {
    let mut positions = Vec::with_capacity(story_ids.len());
    for &story_id in story_ids {
        let item = crate::session::priority::IceboxItem::Story { number: story_id };
        let current_idx = priority.icebox_unified.iter().position(|x| x == &item);
        let bottom_marker_idx = priority
            .icebox_unified
            .iter()
            .enumerate()
            .rev()
            .find(|(_, x)| matches!(x, crate::session::priority::IceboxItem::Marker { .. }))
            .map(|(i, _)| i);
        match (current_idx, bottom_marker_idx) {
            (None, _) => continue,
            (Some(idx), Some(marker_idx)) if idx > marker_idx => {
                positions.push(StoryPosition {
                    id: story_id,
                    position: idx + 1,
                });
            }
            (Some(idx), None) => {
                positions.push(StoryPosition {
                    id: story_id,
                    position: idx + 1,
                });
            }
            (Some(_), Some(_)) => {
                priority.icebox_unified.retain(|x| x != &item);
                priority.icebox_unified.push(item);
                positions.push(StoryPosition {
                    id: story_id,
                    position: priority.icebox_unified.len(),
                });
            }
        }
    }
    positions
}
