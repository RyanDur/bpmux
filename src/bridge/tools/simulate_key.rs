use super::helpers::post_simulate;

pub(in crate::bridge) async fn handle_simulate_key(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("missing 'key'")?;
    // Forward `modifiers` if the caller supplied it. Accept an
    // array of strings; let the REST endpoint validate names so the
    // bridge doesn't need to know the modifier vocabulary.
    let mut body = serde_json::json!({"key": key});
    if let Some(modifiers) = args.get("modifiers") {
        body["modifiers"] = modifiers.clone();
    }
    post_simulate(port, "/api/simulate/key", &body).await
}
