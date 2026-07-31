#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{IterationView, OutcomeView};
use product_management::iteration::{Iteration, IterationId};
use product_management::outcome::{Outcome, OutcomeId, OutcomeLifecycle};
use product_management::story::{Horizon, StoryId};
use workflow::bundled_default::bundled_default;

fn fixed_ts() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-04-26T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}

fn sample_iteration() -> Iteration {
    Iteration {
        id: IterationId::new(42),
        title: "Vela".to_string(),
        body: Some("iteration overview".to_string()),
        filed_at: fixed_ts(),
        filed_by: "pm".to_string(),
        outcome: Some(OutcomeId::new(7)),
        iteration_order: Some(3),
        stories: vec![StoryId::new(100), StoryId::new(101)],
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }
}

fn assert_exact_wire_keys(
    obj: &serde_json::Map<String, serde_json::Value>,
    shape: &str,
    expected: &[&str],
) {
    let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    for required in expected {
        assert!(
            keys.contains(required),
            "{shape} wire must carry `{required}` (a relocation or rename must not drop it); got {keys:?}"
        );
    }
    assert_eq!(
        obj.len(),
        expected.len(),
        "no spurious {shape} keys; got {keys:?}"
    );
}

fn sample_outcome() -> Outcome {
    Outcome {
        id: OutcomeId::new(7),
        title: "Ship the thing".to_string(),
        body: Some("outcome body".to_string()),
        filed_at: fixed_ts(),
        filed_by: "pm".to_string(),
        lifecycle: OutcomeLifecycle::Active {
            horizon: Some(Horizon::Next),
            horizon_order: Some(2),
        },
        iterations: vec![IterationId::new(42)],
    }
}

#[test]
fn iteration_view_wire_shape_is_pinned() {
    let workflow = bundled_default();
    let view = IterationView::project(&sample_iteration(), &[sample_outcome()], &workflow);
    let json = serde_json::to_value(view).unwrap();

    let obj = json
        .as_object()
        .expect("IterationView serializes to an object");
    assert_exact_wire_keys(
        obj,
        "IterationView",
        &[
            "id",
            "title",
            "iteration_order",
            "is_current",
            "complete",
            "builds_outcome",
            "outcome_position",
            "stories",
            "readiness",
        ],
    );

    assert_eq!(json["id"].as_u64(), Some(42));
    assert_eq!(
        json["builds_outcome"].as_u64(),
        Some(7),
        "builds_outcome is the OutcomeId as a bare number (entity_id! is #[serde(transparent)])"
    );
    assert_eq!(
        json["outcome_position"].as_u64(),
        Some(1),
        "iteration 42 is position 1 within outcome 7's ordered iteration list"
    );
    assert_eq!(json["stories"], serde_json::json!([100, 101]));

    let readiness = json["readiness"]
        .as_object()
        .expect("readiness is a nested object");
    assert_exact_wire_keys(
        readiness,
        "IterationReadiness",
        &["drafted", "stories_ordered", "review_closed", "dev_ok"],
    );
}

#[test]
fn outcome_view_wire_shape_is_pinned() {
    let workflow = bundled_default();
    let view = OutcomeView::project(&sample_outcome(), &[sample_iteration()], &workflow);
    let json = serde_json::to_value(view).unwrap();

    let obj = json
        .as_object()
        .expect("OutcomeView serializes to an object");
    assert_exact_wire_keys(
        obj,
        "OutcomeView",
        &[
            "id",
            "title",
            "body",
            "horizon",
            "horizon_order",
            "outcome_readiness",
            "iterations",
        ],
    );

    assert_eq!(json["id"].as_u64(), Some(7));
    assert_eq!(
        json["horizon"].as_str(),
        Some("next"),
        "horizon serializes as the canonical band string"
    );
    assert_eq!(json["horizon_order"].as_u64(), Some(2));

    let readiness = json["outcome_readiness"]
        .as_object()
        .expect("outcome_readiness is a nested object");
    assert_exact_wire_keys(
        readiness,
        "OutcomeReadiness",
        &["iteration_count", "gap_count", "all_ready"],
    );

    let iterations = json["iterations"]
        .as_array()
        .expect("iterations is a nested array");
    assert_eq!(iterations.len(), 1, "outcome 7 builds exactly iteration 42");
    let building = iterations[0]
        .as_object()
        .expect("BuildingIterationView is a nested object");
    assert_exact_wire_keys(
        building,
        "BuildingIterationView",
        &["id", "name", "position", "readiness"],
    );
    assert_eq!(building["id"].as_u64(), Some(42));
    assert_eq!(building["name"].as_str(), Some("Vela"));
    assert_eq!(building["position"].as_u64(), Some(1));
}
