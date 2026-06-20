#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::roadmap::{render_drill, render_overview};
use serde_json::json;

#[test]
fn overview_renders_each_state_in_its_band() {
    let out = render_overview(&json!({
        "bands": [
            {"label": "Now", "outcomes": [
                {"number": 12, "name": "A", "state": "ready", "gap_count": 0},
                {"number": 13, "name": "B", "state": "needs-attention", "gap_count": 2}
            ]},
            {"label": "Next", "outcomes": [
                {"number": 34, "name": "C", "state": "planned", "gap_count": 0}
            ]},
            {"label": "Later", "outcomes": [
                {"number": 40, "name": "D", "state": "done", "gap_count": 0}
            ]}
        ]
    }));
    assert!(
        out.starts_with("ROADMAP\n"),
        "header is the substrate-view, not markdown"
    );
    assert!(out.contains("Now") && out.contains("#12  A  ready"));
    assert!(
        out.contains("#13  B  2 needs-attention"),
        "gap-count rendered"
    );
    assert!(out.contains("#34  C  planned · iterations TBD"));
    assert!(out.contains("#40  D  done"));
}

#[test]
fn overview_empty_is_the_substrate_views_own_marker_never_markdown() {
    assert_eq!(
        render_overview(&json!({ "bands": [] })),
        "ROADMAP\n  (no outcomes yet)\n"
    );
}

#[test]
fn drill_renders_header_convene_iterations_and_checks() {
    let out = render_drill(&json!({
        "outcome_number": 34,
        "outcome_name": "Decision-layer",
        "drill": {
            "convene": {
                "next_iteration_name": "Crux-1740",
                "next_gap_count": 2,
                "active_gap_label": "stories-ordered",
                "then_iteration_name": "Crux-1741"
            },
            "iterations": [
                {"order": 1, "number": 1740, "name": "Crux-1740", "state": "next",
                 "gap_count": 2, "checks": [
                    {"label": "drafted", "state": "met"},
                    {"label": "stories-ordered", "state": "gap"},
                    {"label": "designer-review", "state": "pending"},
                    {"label": "dev-OK", "state": "pending"}
                ]},
                {"order": 2, "number": 1741, "name": "Crux-1741", "state": "not-started",
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
