#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use chrono::{TimeZone, Utc};
use product_management::outcome::{Outcome, OutcomeId, OutcomeLifecycle};
use tower::ServiceExt;

use super::response_json;
use crate::store::SeededStore;

fn active_outcome(id: u64, title: &str) -> Outcome {
    Outcome {
        id: OutcomeId::new(id),
        title: title.to_string(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 7, 16, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        lifecycle: OutcomeLifecycle::Active {
            horizon: None,
            horizon_order: None,
        },
        iterations: Vec::new(),
    }
}

fn idle_orchestrator() -> crate::sync::OrchestratorHandle {
    Some(
        crate::store::build_seeded_orch(
            Arc::new(crate::integrations::in_memory::InMemorySourceAdapter::new()),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
            vec![],
            vec![],
            vec![],
        )
        .0,
    )
}

async fn roadmap_body(seeded: &SeededStore) -> serde_json::Value {
    let router = axum::Router::new()
        .route(
            "/api/roadmap",
            axum::routing::get(crate::transport::rest::get_roadmap),
        )
        .layer(axum::Extension(idle_orchestrator()))
        .layer(axum::Extension(seeded.select()));
    let request = Request::builder()
        .method("GET")
        .uri("/api/roadmap")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    response_json(response).await
}

#[tokio::test]
async fn the_roadmap_view_derives_its_outcomes_from_the_central_store() {
    let seeded = SeededStore::new(
        vec![
            active_outcome(701, "Agents read the board"),
            active_outcome(702, "The roadmap tells the truth"),
        ],
        Vec::new(),
        Vec::new(),
    );

    let body = roadmap_body(&seeded).await;

    let names: Vec<String> = body["outcomes"]
        .as_array()
        .expect("the roadmap view returns an outcomes array")
        .iter()
        .map(|card| card["name"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        names.contains(&"Agents read the board".to_string())
            && names.contains(&"The roadmap tells the truth".to_string()),
        "every outcome seeded into the central store reaches the roadmap view; got {names:?}"
    );
    assert_eq!(
        body["total"], 2,
        "the roadmap's total counts what the store holds"
    );
}

#[tokio::test]
async fn the_roadmap_view_of_an_empty_store_is_empty_rather_than_absent() {
    let seeded = SeededStore::new(Vec::new(), Vec::new(), Vec::new());

    let body = roadmap_body(&seeded).await;

    assert_eq!(
        body["outcomes"].as_array().map(Vec::len),
        Some(0),
        "an empty store yields an empty outcome list, not a missing field"
    );
    assert_eq!(body["total"], 0);
}
