#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::rerun_ci::*;

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
