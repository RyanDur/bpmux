use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tower::ServiceExt;

use super::response_json;
use crate::room::LivePtyRegistry;
use crate::routing::RoutingStore;
use crate::transport::rest::post_terminal_spawn;

fn spawn_app(store_dir: &TempDir) -> (axum::Router, Arc<LivePtyRegistry>, Arc<RoutingStore>) {
    let registry = Arc::new(LivePtyRegistry::new());
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let session = Arc::new(crate::session::Session::new(
        store_dir.path().to_path_buf(),
        "owner".to_string(),
        tx,
    ));
    let routing = Arc::new(RoutingStore::new(session.storage.clone()));
    let router = axum::Router::new()
        .route(
            "/api/terminals/spawn",
            axum::routing::post(post_terminal_spawn),
        )
        .layer(axum::Extension(registry.clone()))
        .layer(axum::Extension(0u16))
        .layer(axum::Extension(routing.clone()))
        .with_state(session);
    (router, registry, routing)
}

#[tokio::test]
async fn the_plus_spawn_flow_auto_registers_an_agent_running_in_its_working_dir_headless() {
    let store_dir = TempDir::new().expect("store dir");
    let (router, registry, routing) = spawn_app(&store_dir);

    let work = TempDir::new().expect("work dir");
    let work_path = work.path().to_string_lossy().to_string();
    let work_name = work
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let request = Request::builder()
        .method("POST")
        .uri("/api/terminals/spawn")
        .body(Body::empty())
        .expect("request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let spawned = response_json(response).await;
    let terminal_id = spawned["terminal_id"]
        .as_str()
        .expect("terminal_id")
        .to_string();

    registry.write_to(&terminal_id, format!("cd {work_path}\n").as_bytes());
    registry.write_to(&terminal_id, b"sleep 4\n");

    let marker = work.path().join(".bpmux-session-id");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while std::time::Instant::now() < deadline {
        if marker.is_file() && !routing.resolve(&work_name).is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    assert!(
        marker.is_file(),
        "the ＋-spawned terminal's detector minted the work-dir's marker on a sustained foreground-job — the handler wires detection for a bare shell, no BPMUX_ROOM_AGENT gate"
    );
    let session = crate::session::read_or_create_dir_session(work.path()).expect("read marker");
    assert_eq!(
        routing.resolve(&work_name),
        vec![session.id],
        "the agent auto-registered HEADLESS through the real spawn endpoint (no viewer subscribed) — routable by the folder basename, the agnostic-registration WALK-NODE end-to-end"
    );

    if let Some(live) = registry.remove(&terminal_id) {
        live.close_and_reap();
    }
}
