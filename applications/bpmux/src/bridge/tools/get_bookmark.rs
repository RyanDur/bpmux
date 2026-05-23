use super::helpers::get_session_dir;

pub(in crate::bridge) async fn handle_get_bookmark(
    args: &serde_json::Map<String, serde_json::Value>,
    _port: u16,
    name: &str,
) -> Result<String, String> {
    use crate::session::bookmark::load_bookmark;
    let session_dir = get_session_dir().await?;
    let default_role = name.split(':').next().unwrap_or(name);
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or(default_role);
    let bookmark = load_bookmark(&session_dir, role);
    let view = crate::bridge::view_model::BookmarkView::from(&bookmark);
    serde_json::to_string_pretty(&view).map_err(|e| format!("serialize: {e}"))
}
