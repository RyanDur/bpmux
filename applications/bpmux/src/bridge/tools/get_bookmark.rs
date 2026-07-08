use super::helpers::get_data_dir;

pub(in crate::bridge) async fn handle_get_bookmark(
    args: &serde_json::Map<String, serde_json::Value>,
    _port: u16,
    name: &str,
) -> Result<String, String> {
    use crate::bookmark::load_bookmark;
    let data_dir = get_data_dir().await?;
    let default_role = name.split(':').next().unwrap_or(name);
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or(default_role);
    let bookmark = load_bookmark(&data_dir, role);
    let view = crate::bridge::view_model::BookmarkView::from(&bookmark);
    serde_json::to_string_pretty(&view).map_err(|e| format!("serialize: {e}"))
}
