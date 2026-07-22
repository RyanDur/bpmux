use super::helpers::get_data_dir;

pub async fn handle_save_bookmark(
    args: &serde_json::Map<String, serde_json::Value>,
    _port: u16,
    name: &str,
) -> Result<String, String> {
    use bookmark::{Bookmark, save_bookmark};
    let data_dir = get_data_dir().await?;
    let bookmark = Bookmark {
        current_task: args
            .get("current_task")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        decisions: args
            .get("decisions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        open_questions: args
            .get("open_questions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        next_step: args
            .get("next_step")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        notes: args
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        last_updated: chrono::Utc::now().to_rfc3339(),
    };
    let role = name.split(':').next().unwrap_or(name);
    save_bookmark(&data_dir, role, &bookmark).map_err(|e| format!("save failed: {e}"))?;
    Ok("bookmark saved".to_string())
}
