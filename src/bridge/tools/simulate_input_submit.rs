use super::helpers::post_simulate;

pub(in crate::bridge) async fn handle_simulate_input_submit(port: u16) -> Result<String, String> {
    post_simulate(port, "/api/simulate/input_submit", &serde_json::json!({})).await
}
