#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::roadmap::render_drill;
use serde_json::json;

#[test]
fn drill_renders_header_convene_iterations_and_checks() {
    let out = render_drill(&json!({
        "outcome_id": 34,
        "outcome_name": "Decision-layer",
        "drill": {
            "convene": {
                "next_iteration_name": "Crux-1740",
                "next_gap_count": 2,
                "active_gap_label": "stories-ordered",
                "then_iteration_name": "Crux-1741"
            },
            "iterations": [
                {"order": 1, "id": 1740, "name": "Crux-1740", "state": "next",
                 "gap_count": 2, "checks": [
                    {"label": "drafted", "state": "met"},
                    {"label": "stories-ordered", "state": "gap"},
                    {"label": "designer-review", "state": "pending"},
                    {"label": "dev-OK", "state": "pending"}
                ]},
                {"order": 2, "id": 1741, "name": "Crux-1741", "state": "not-started",
                 "gap_count": 4, "checks": []}
            ]
        }
    }));
    assert!(out.starts_with("#34 Decision-layer\n"));
    assert!(out.contains("convene? next: Crux-1740 — 2 gaps · then Crux-1741"));
    assert!(out.contains("1. #1740 Crux-1740") && out.contains("[next]") && out.contains("2 gaps"));
    assert!(
        out.contains(
            "drafted met · stories-ordered GAP · designer-review pending · dev-OK pending"
        )
    );
    assert!(out.contains("#1741 Crux-1741") && out.contains("[not-started]"));
    assert!(
        !out.contains("[not-started]   4 gaps"),
        "a not-started iteration shows no gap-count + no checks line"
    );
}
