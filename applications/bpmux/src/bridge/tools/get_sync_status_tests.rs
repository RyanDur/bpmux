#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::get_sync_status::handle_get_sync_status;

#[tokio::test]
async fn surfaces_unreachable_server_as_error() {
    let result = handle_get_sync_status(1).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("electron-server unreachable") || err.contains("auth-token"),
        "expected unreachable-server or auth-token error, got: {err}"
    );
}
