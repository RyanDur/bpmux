use serde_json::{Map, Value};

pub(super) const RPC_AGENT_HEADER: &str = "X-BPMUX-Agent";

fn load_auth_token() -> Result<String, String> {
    let path = crate::auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    crate::auth::read(&path).map_err(|e| format!("read auth-token from {}: {e}", path.display()))
}

pub(super) async fn post_rpc(
    port: u16,
    tool: &str,
    agent: &str,
    body: &Value,
    token: &str,
) -> Result<String, String> {
    let url = crate::transport::local_url::api_url(port, &format!("/api/rpc/{tool}"));
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header(RPC_AGENT_HEADER, agent)
        .json(body)
        .send()
        .await
        .map_err(|e| {
            format!("rpc {tool}: electron-server unreachable at {url}: {e} — bridge requires the canonical store; ensure Electron is running")
        })?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status.is_success() {
        parsed
            .get("message")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| {
                format!(
                    "rpc {tool}: success status {status} but response missing 'message' field: {text}"
                )
            })
    } else {
        let err = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or(text.as_str())
            .to_string();
        Err(format!("rpc {tool}: {status} — {err}"))
    }
}

pub(super) async fn dispatch_via_rpc(
    port: u16,
    tool: &str,
    agent: &str,
    args: &Map<String, Value>,
) -> Result<String, String> {
    let body = Value::Object(args.clone());
    let token = load_auth_token()?;
    post_rpc(port, tool, agent, &body, &token).await
}
