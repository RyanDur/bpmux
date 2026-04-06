use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

#[derive(Deserialize)]
pub struct PostMessage {
    pub from: String,
    pub text: String,
    pub message_type: Option<String>,
}

#[derive(Deserialize)]
pub struct GetMessagesParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub async fn get_messages(
    State(session): State<Arc<session::Session>>,
    Query(params): Query<GetMessagesParams>,
) -> impl IntoResponse {
    match session.read_messages() {
        Ok(messages) => {
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit.unwrap_or(20);
            let page: Vec<_> = messages
                .into_iter()
                .rev()
                .skip(offset)
                .take(limit)
                .collect();
            (
                StatusCode::OK,
                serde_json::to_string(&page).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read: {e}"),
        ),
    }
}

pub async fn post_message(
    State(session): State<Arc<session::Session>>,
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
        message_type: payload.message_type,
        to: None,
    };
    if let Err(e) = session.broadcast(&msg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to broadcast: {e}"),
        );
    }
    (StatusCode::CREATED, "message sent".to_string())
}

// --- Pin endpoints ---

pub async fn get_pins(State(session): State<Arc<session::Session>>) -> impl IntoResponse {
    match session.read_pins() {
        Ok(pins) => {
            let numbered: Vec<serde_json::Value> = pins
                .iter()
                .enumerate()
                .map(|(i, p)| serde_json::json!({"index": i + 1, "text": p}))
                .collect();
            (
                StatusCode::OK,
                serde_json::to_string(&numbered).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read pins: {e}"),
        ),
    }
}

#[derive(Deserialize)]
pub struct AddPinRequest {
    pub text: String,
    pub from: Option<String>,
}

pub async fn add_pin(
    State(session): State<Arc<session::Session>>,
    Json(payload): Json<AddPinRequest>,
) -> impl IntoResponse {
    if payload.text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "text must not be empty".to_string(),
        );
    }
    match session.add_pin(payload.text.clone()) {
        Ok(Ok(pins)) => {
            let pin_num = pins.len();
            // Broadcast system message about the pin
            let author = payload
                .from
                .as_deref()
                .and_then(|f| f.split(':').next())
                .unwrap_or("someone");
            let sys_msg = session::Message {
                from: "bpmux".to_string(),
                text: format!("{author} pinned #{pin_num}: {}", payload.text),
                ts: chrono::Utc::now(),
                message_type: Some("system".to_string()),
                to: None,
            };
            let _ = session.broadcast(&sys_msg);
            (
                StatusCode::CREATED,
                format!("pinned #{pin_num}: {}", payload.text),
            )
        }
        Ok(Err(msg)) => (StatusCode::CONFLICT, msg.to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to add pin: {e}"),
        ),
    }
}

#[derive(Deserialize)]
pub struct RemovePinParams {
    pub from: Option<String>,
}

pub async fn remove_pin(
    State(session): State<Arc<session::Session>>,
    axum::extract::Path(index): axum::extract::Path<usize>,
    Query(params): Query<RemovePinParams>,
) -> impl IntoResponse {
    match session.remove_pin(index) {
        Ok(Ok(pins)) => {
            let author = params
                .from
                .as_deref()
                .and_then(|f| f.split(':').next())
                .unwrap_or("someone");
            // The removed pin text isn't in the returned vec, but we can report the index
            let _ = pins; // pins is the updated list after removal
            let sys_msg = session::Message {
                from: "bpmux".to_string(),
                text: format!("{author} unpinned #{index}"),
                ts: chrono::Utc::now(),
                message_type: Some("system".to_string()),
                to: None,
            };
            let _ = session.broadcast(&sys_msg);
            (StatusCode::OK, format!("unpinned #{index}"))
        }
        Ok(Err(msg)) => (StatusCode::NOT_FOUND, msg),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to remove pin: {e}"),
        ),
    }
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
        let session = Arc::new(session::Session::new(dir.path().to_path_buf(), tx.clone()));
        let router = axum::Router::new()
            .route(
                "/api/messages",
                axum::routing::get(get_messages).post(post_message),
            )
            .route("/api/pins", axum::routing::get(get_pins).post(add_pin))
            .route("/api/pins/{index}", axum::routing::delete(remove_pin))
            .with_state(session);
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

    fn seed_messages(dir: &TempDir, count: usize) -> Arc<session::Session> {
        let (tx, _rx) = broadcast::channel(16);
        let session = Arc::new(session::Session::new(dir.path().to_path_buf(), tx));
        for i in 0..count {
            let msg = session::Message {
                from: "user".to_string(),
                text: format!("msg {i}"),
                ts: chrono::Utc::now(),
                message_type: None,
                to: None,
            };
            session.broadcast(&msg).expect("broadcast");
        }
        session
    }

    #[tokio::test]
    async fn gets_messages_with_limit() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let session = seed_messages(&dir, 5);
        let router = axum::Router::new()
            .route("/api/messages", axum::routing::get(get_messages))
            .with_state(session);
        let request = Request::builder()
            .method("GET")
            .uri("/api/messages?limit=2")
            .body(Body::empty())
            .expect("build request");
        let response = router.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(msgs.len(), 2);
        // Most recent first
        assert_eq!(msgs[0]["text"], "msg 4");
        assert_eq!(msgs[1]["text"], "msg 3");
    }

    #[tokio::test]
    async fn gets_messages_with_offset() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let session = seed_messages(&dir, 5);
        let router = axum::Router::new()
            .route("/api/messages", axum::routing::get(get_messages))
            .with_state(session);
        let request = Request::builder()
            .method("GET")
            .uri("/api/messages?limit=2&offset=2")
            .body(Body::empty())
            .expect("build request");
        let response = router.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(msgs.len(), 2);
        // Reversed: [4,3,2,1,0], skip 2 -> [2,1,0], take 2 -> [2,1]
        assert_eq!(msgs[0]["text"], "msg 2");
        assert_eq!(msgs[1]["text"], "msg 1");
    }

    #[tokio::test]
    async fn defaults_to_twenty_messages() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let session = seed_messages(&dir, 25);
        let router = axum::Router::new()
            .route("/api/messages", axum::routing::get(get_messages))
            .with_state(session);
        let request = Request::builder()
            .method("GET")
            .uri("/api/messages")
            .body(Body::empty())
            .expect("build request");
        let response = router.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(msgs.len(), 20);
    }

    // --- Pin endpoint tests ---

    #[tokio::test]
    async fn post_pin_and_get_pins() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let (app, _tx) = test_app(&dir);

        // POST a pin
        let request = Request::builder()
            .method("POST")
            .uri("/api/pins")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"text":"channels-as-views","from":"dev:claude-code"}"#,
            ))
            .expect("build request");
        let response = app.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::CREATED);

        // GET pins
        let request = Request::builder()
            .method("GET")
            .uri("/api/pins")
            .body(Body::empty())
            .expect("build request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let pins: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0]["index"], 1);
        assert_eq!(pins[0]["text"], "channels-as-views");
    }

    #[tokio::test]
    async fn post_pin_rejects_duplicate() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let (app, _tx) = test_app(&dir);

        let body = r#"{"text":"dup test","from":"dev:claude-code"}"#;
        let req = || {
            Request::builder()
                .method("POST")
                .uri("/api/pins")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .expect("build request")
        };

        let response = app.clone().oneshot(req()).await.expect("response");
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app.oneshot(req()).await.expect("response");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn delete_pin_removes_by_index() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let (app, _tx) = test_app(&dir);

        // Add a pin
        let request = Request::builder()
            .method("POST")
            .uri("/api/pins")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"text":"to remove","from":"dev:claude-code"}"#,
            ))
            .expect("build request");
        app.clone().oneshot(request).await.expect("response");

        // Delete pin #1
        let request = Request::builder()
            .method("DELETE")
            .uri("/api/pins/1?from=dev:claude-code")
            .body(Body::empty())
            .expect("build request");
        let response = app.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        // Verify empty
        let request = Request::builder()
            .method("GET")
            .uri("/api/pins")
            .body(Body::empty())
            .expect("build request");
        let response = app.oneshot(request).await.expect("response");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let pins: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(pins.len(), 0);
    }

    #[tokio::test]
    async fn delete_pin_invalid_index_returns_404() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let (app, _tx) = test_app(&dir);

        let request = Request::builder()
            .method("DELETE")
            .uri("/api/pins/99")
            .body(Body::empty())
            .expect("build request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn pin_broadcasts_system_message() {
        let dir = TempDir::new().expect("tmpdir");
        init_session(&dir);
        let (app, tx) = test_app(&dir);
        let mut rx = tx.subscribe();

        let request = Request::builder()
            .method("POST")
            .uri("/api/pins")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"text":"broadcast test","from":"dev:claude-code"}"#,
            ))
            .expect("build request");
        app.oneshot(request).await.expect("response");

        let msg = rx.try_recv().expect("broadcast message");
        assert!(msg.text.contains("dev pinned #1: broadcast test"));
        assert_eq!(msg.message_type.as_deref(), Some("system"));
    }
}
