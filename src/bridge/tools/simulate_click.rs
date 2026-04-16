use super::helpers::post_simulate;

pub(in crate::bridge) async fn handle_simulate_click(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let x = args
        .get("x")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'x'")? as u16;
    let y = args
        .get("y")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'y'")? as u16;
    post_simulate(
        port,
        "/api/simulate/click",
        &serde_json::json!({"x": x, "y": y}),
    )
    .await
}
