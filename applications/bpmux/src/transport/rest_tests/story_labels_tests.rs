#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use chrono::{TimeZone, Utc};
use product_management::iteration::{Iteration, IterationAction, IterationId};
use tokio::sync::mpsc::UnboundedReceiver;
use tower::ServiceExt;

use crate::hub;
use crate::store::{AppAction, Dispatch, SeededStore};

fn fresh_test_hub() -> (Arc<hub::Hub>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let (wake_tx, _wake_rx) = tokio::sync::broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        wake_tx,
    ));
    (hub, dir)
}

fn seeded_iteration(id: u64) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: "Iteration Carina".to_string(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 7, 16, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        complete: false,
        is_current: false,
        iteration_order: None,
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        stories: Vec::new(),
        outcome: None,
    }
}

fn upserted_iteration(rx: &mut UnboundedReceiver<AppAction>, id: u64) -> Option<Iteration> {
    let mut found = None;
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Iteration(IterationAction::Upserted(iterations)) = action
            && let Some(iteration) = iterations
                .into_iter()
                .find(|iteration| iteration.id == IterationId::new(id))
        {
            found = Some(iteration);
        }
    }
    found
}

async fn post_labels(
    seeded: &SeededStore,
    dispatch: Dispatch,
    id: u64,
    body: &str,
) -> axum::http::StatusCode {
    let orchestrator: crate::sync::OrchestratorHandle =
        Some(Arc::new(crate::sync::SyncOrchestrator::new(
            Arc::new(crate::integrations::in_memory::InMemorySourceAdapter::new()),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        )));
    let (hub, _hub_dir) = fresh_test_hub();
    let router = axum::Router::new()
        .route(
            "/api/stories/{number}/labels",
            axum::routing::post(crate::transport::rest::post_story_labels),
        )
        .with_state(hub)
        .layer(axum::Extension(orchestrator))
        .layer(axum::Extension(dispatch))
        .layer(axum::Extension(seeded.select()));
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/stories/{id}/labels"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    router.oneshot(request).await.expect("response").status()
}

#[tokio::test]
async fn labelling_an_iteration_current_designates_it_on_the_iteration_entity() {
    let seeded = SeededStore::new(Vec::new(), vec![seeded_iteration(1400)], Vec::new());
    let (dispatch, mut rx) = Dispatch::recording();

    let status = post_labels(
        &seeded,
        dispatch,
        1400,
        r#"{"add":["current-iteration"],"remove":[]}"#,
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::OK);
    let iteration = upserted_iteration(&mut rx, 1400)
        .expect("labelling an iteration story upserts the iteration it decodes to");
    assert!(
        iteration.is_current,
        "the current-iteration label designates the iteration the store already holds"
    );
}

#[tokio::test]
async fn removing_the_current_label_clears_the_designation() {
    let mut current = seeded_iteration(1400);
    current.is_current = true;
    let seeded = SeededStore::new(Vec::new(), vec![current], Vec::new());
    let (dispatch, mut rx) = Dispatch::recording();

    let status = post_labels(
        &seeded,
        dispatch,
        1400,
        r#"{"add":[],"remove":["current-iteration"]}"#,
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::OK);
    let iteration = upserted_iteration(&mut rx, 1400).expect("the iteration is upserted");
    assert!(
        !iteration.is_current,
        "removing the label clears the designation rather than leaving it stale"
    );
}

#[tokio::test]
async fn labelling_a_story_the_store_holds_no_iteration_for_upserts_no_iteration() {
    let seeded = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    let (dispatch, mut rx) = Dispatch::recording();

    let status = post_labels(
        &seeded,
        dispatch,
        999,
        r#"{"add":["current-iteration"],"remove":[]}"#,
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(
        upserted_iteration(&mut rx, 999).is_none(),
        "an id the store holds no iteration for is labelled without inventing one"
    );
}
