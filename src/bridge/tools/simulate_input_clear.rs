use super::helpers::post_simulate;

pub(in crate::bridge) async fn handle_simulate_input_clear(port: u16) -> Result<String, String> {
    post_simulate(port, "/api/simulate/input_clear", &serde_json::json!({})).await
}
