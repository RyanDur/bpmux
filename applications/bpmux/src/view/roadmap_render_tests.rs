#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::roadmap_render::{render_overview_layer, render_roadmap};
use crate::product_management::iteration::readiness::OutcomeReadiness;
use crate::product_management::outcome::OutcomeId;
use crate::product_management::roadmap::selectors::{RoadmapOutcomeInput, derive_roadmap};
use crate::product_management::story::Horizon;

fn outcome(id: u64, title: &str, horizon: Option<Horizon>) -> RoadmapOutcomeInput {
    RoadmapOutcomeInput {
        id: OutcomeId::new(id),
        title: title.to_string(),
        body: String::new(),
        horizon,
        horizon_order: None,
        done: false,
        readiness: None,
    }
}

fn with_order(mut o: RoadmapOutcomeInput, order: Option<u32>) -> RoadmapOutcomeInput {
    o.horizon_order = order;
    o
}

fn readiness(iteration_count: u32, gap_count: u32, all_ready: bool) -> OutcomeReadiness {
    OutcomeReadiness {
        iteration_count,
        gap_count,
        all_ready,
    }
}

#[test]
fn render_labels_each_band_once_and_shows_per_outcome_state() {
    let mut now_ready = with_order(outcome(42, "Own infra", Some(Horizon::Now)), Some(1));
    now_ready.readiness = Some(readiness(1, 0, true));
    let mut now_gap = with_order(outcome(10, "Another now", Some(Horizon::Now)), Some(2));
    now_gap.readiness = Some(readiness(2, 2, false));
    let next_planned = outcome(7, "A next thing", Some(Horizon::Next));

    let text = render_roadmap(&derive_roadmap(&[now_ready, now_gap, next_planned]));
    let lines: Vec<&str> = text.lines().collect();

    assert_eq!(lines[0], "ROADMAP");
    assert!(lines[1].trim_start().starts_with("Now"));
    assert!(lines[1].contains("#42") && lines[1].ends_with("ready"));
    assert!(
        !lines[2].trim_start().starts_with("Now"),
        "the second Now outcome omits the repeated band label: {:?}",
        lines[2]
    );
    assert!(lines[2].contains("#10") && lines[2].ends_with("2 needs-attention"));
    assert!(lines[3].trim_start().starts_with("Next"));
    assert!(lines[3].contains("#7") && lines[3].ends_with("planned · iterations TBD"));
}

#[test]
fn render_shows_the_gap_count_with_the_needs_attention_state() {
    let mut one_gap = outcome(5, "Almost", Some(Horizon::Now));
    one_gap.readiness = Some(readiness(1, 1, false));
    let line = render_roadmap(&derive_roadmap(&[one_gap]));
    let outcome_line = line.lines().nth(1).unwrap();
    assert!(
        outcome_line.ends_with("1 needs-attention"),
        "got: {outcome_line:?}"
    );
}

#[test]
fn render_handles_no_outcomes() {
    assert_eq!(render_roadmap(&[]), "ROADMAP\n  (no outcomes yet)\n");
}

#[test]
fn overview_layer_renders_served_cards_via_the_shared_renderer() {
    let out = render_overview_layer(&serde_json::json!({
        "outcomes": [
            {"id": 12, "name": "A", "why": {}, "horizon": "now", "state": "ready", "gap_count": 0, "tone": "ready"},
            {"id": 13, "name": "B", "why": {}, "horizon": "now", "state": "needs-attention", "gap_count": 2, "tone": "gap"},
            {"id": 40, "name": "D", "why": {}, "horizon": "later", "state": "done", "gap_count": 0, "tone": "ready"}
        ],
        "total": 3,
        "shown": 3
    }));
    assert!(out.starts_with("ROADMAP\n"));
    assert!(out.contains("Now") && out.contains("#12  A  ready"));
    assert!(out.contains("#13  B  2 needs-attention"));
    assert!(out.contains("#40  D  done"));
}

#[test]
fn overview_layer_labels_the_elided_count_when_a_limit_dropped_rows() {
    let out = render_overview_layer(&serde_json::json!({
        "outcomes": [
            {"id": 12, "name": "A", "why": {}, "horizon": "now", "state": "ready", "gap_count": 0, "tone": "ready"}
        ],
        "total": 12,
        "shown": 1
    }));
    assert!(
        out.contains("showing 1 of 12"),
        "a limited view labels what it elided; got {out:?}"
    );
}

#[test]
fn overview_layer_omits_the_elision_label_when_nothing_was_dropped() {
    let out = render_overview_layer(&serde_json::json!({
        "outcomes": [
            {"id": 12, "name": "A", "why": {}, "horizon": "now", "state": "ready", "gap_count": 0, "tone": "ready"}
        ],
        "total": 1,
        "shown": 1
    }));
    assert!(!out.contains("showing"), "no elision label; got {out:?}");
}
