pub(in crate::bridge) async fn handle_get_identity(
    args: &serde_json::Map<String, serde_json::Value>,
    default_role: &str,
) -> Result<String, String> {
    use memory::identity::{default_identity_dir, load_identity};
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or(default_role);
    let base = default_identity_dir().map_err(|e| format!("identity dir: {e}"))?;
    let index = load_identity(&base, role);
    let view = crate::bridge::view_model::IdentityIndexView::from(&index);
    serde_json::to_string_pretty(&view).map_err(|e| format!("serialize: {e}"))
}
