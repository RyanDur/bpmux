use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::ws::WsState;
use crate::session;

#[derive(Deserialize)]
pub struct PostMessage {
    pub from: String,
    pub text: String,
}

pub async fn post_message(
    State(state): State<Arc<WsState>>,
    Json(payload): Json<PostMessage>,
) -> impl IntoResponse {
    if payload.text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "text must not be empty".to_string(),
        );
    }

    let msg = session::Message {
        from: payload.from,
        text: payload.text,
        ts: chrono::Utc::now(),
    };

    if let Err(e) = session::broadcast_message(&state.session_dir, &msg, &state.wake_tx) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to broadcast: {e}"),
        );
    }

    (StatusCode::CREATED, "message sent".to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::sync::broadcast;
    use tower::ServiceExt;

    fn test_app(dir: &TempDir) -> (axum::Router, broadcast::Sender<session::Message>) {
        let (tx, _rx) = broadcast::channel(16);
        let state = Arc::new(WsState {
            session_dir: Arc::new(dir.path().to_path_buf()),
            session_id: "test".to_string(),
            wake_tx: tx.clone(),
        });
        let router = axum::Router::new()
            .route("/api/messages", axum::routing::post(post_message))
            .with_state(state);
        (router, tx)
    }

    fn init_session(dir: &TempDir) -> PathBuf {
        let messages_file = dir.path().join("messages-current.jsonl");
        std::fs::write(&messages_file, "").expect("create messages file");
        dir.path().to_path_buf()
    }

    #[tokio::test]
    async fn posts_message_and_broadcasts() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let (app, tx) = test_app(&dir);
        let mut rx = tx.subscribe();

        let request = Request::builder()
            .method("POST")
            .uri("/api/messages")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"from":"test:hook","text":"hello from hook"}"#,
            ))
            .expect("build request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::CREATED);

        let msg = rx.try_recv().expect("broadcast message");
        assert_eq!(msg.from, "test:hook");
        assert_eq!(msg.text, "hello from hook");
    }

    #[tokio::test]
    async fn rejects_empty_text() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let (app, _tx) = test_app(&dir);

        let request = Request::builder()
            .method("POST")
            .uri("/api/messages")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"from":"test:hook","text":""}"#))
            .expect("build request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_missing_fields() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let (app, _tx) = test_app(&dir);

        let request = Request::builder()
            .method("POST")
            .uri("/api/messages")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"text":"no from field"}"#))
            .expect("build request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
