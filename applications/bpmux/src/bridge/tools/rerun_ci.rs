pub(in crate::bridge) async fn handle_rerun_ci(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let run_id = args
        .get("run_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'run_id' argument (u64 expected)")?;
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::auth::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, &format!("/api/ci/rerun/{run_id}"));
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("rerun_ci: electron-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("rerun_ci: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("rerun_ci: {status} — {body}"));
    }
    Ok(body)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_call_without_run_id_arg() {
        let result = handle_rerun_ci(&serde_json::Map::new(), 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing 'run_id'"));
    }

    #[tokio::test]
    async fn rejects_call_with_non_integer_run_id() {
        let mut args = serde_json::Map::new();
        args.insert("run_id".into(), serde_json::json!("not-a-number"));
        let result = handle_rerun_ci(&args, 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing 'run_id'"));
    }

    #[tokio::test]
    async fn surfaces_unreachable_server_as_error() {
        let mut args = serde_json::Map::new();
        args.insert("run_id".into(), serde_json::json!(42));
        let result = handle_rerun_ci(&args, 1).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("electron-server unreachable") || err.contains("auth-token"),
            "expected unreachable-server or auth-token error, got: {err}"
        );
    }
}
