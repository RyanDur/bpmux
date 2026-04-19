use super::helpers::post_simulate;

pub(in crate::bridge) async fn handle_simulate_type(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("missing 'text'")?;
    let body = serde_json::json!({"text": text});
    post_simulate(port, "/api/simulate/type", &body).await
}
