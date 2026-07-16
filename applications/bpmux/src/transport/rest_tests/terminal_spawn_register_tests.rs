use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tower::ServiceExt;

use super::response_json;
use crate::location::test_support::DataBaseGuard;
use crate::room::LivePtyRegistry;
use crate::store::SeededStore;
use crate::transport::rest::post_terminal_spawn;

fn spawn_app(store_dir: &TempDir) -> (axum::Router, Arc<LivePtyRegistry>, SeededStore) {
    let registry = Arc::new(LivePtyRegistry::new());
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let hub = Arc::new(crate::hub::Hub::new(
        store_dir.path().to_path_buf(),
        "owner".to_string(),
        tx,
    ));
    let seeded = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    let router = axum::Router::new()
        .route(
            "/api/terminals/spawn",
            axum::routing::post(post_terminal_spawn),
        )
        .layer(axum::Extension(registry.clone()))
        .layer(axum::Extension(0u16))
        .layer(axum::Extension(seeded.select()))
        .layer(axum::Extension(seeded.dispatcher()))
        .with_state(hub);
    (router, registry, seeded)
}

#[tokio::test]
async fn the_plus_spawn_flow_auto_registers_an_agent_running_in_its_working_dir_headless() {
    let data_base = TempDir::new().expect("registry base");
    let _base_guard = DataBaseGuard::set(data_base.path());

    let store_dir = TempDir::new().expect("store dir");
    let (router, registry, store) = spawn_app(&store_dir);
    let resolve = |display: &str| {
        store
            .select()
            .select(|state| crate::store::routing::resolve(&state.routing, display))
    };

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

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while std::time::Instant::now() < deadline {
        if !resolve(&work_name).is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    assert!(
        !work.path().join(".bpmux-user-id").exists(),
        "the ＋-spawned terminal's detector registered the agent into the data-home registry on a sustained foreground-job — NO marker is written into the work dir; the handler wires detection for a bare shell, no BPMUX_ROOM_AGENT gate"
    );
    let user = crate::location::register_dir_user(work.path()).expect("reclaim from registry");
    assert_eq!(
        resolve(&work_name),
        vec![user.id],
        "the agent auto-registered HEADLESS through the real spawn endpoint (no viewer subscribed) — routable by the folder basename, the agnostic-registration WALK-NODE end-to-end"
    );

    if let Some(live) = registry.remove(&terminal_id) {
        live.close_and_reap();
    }
}
