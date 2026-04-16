#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for the #220 restart-affordance pure helpers.
//!
//! Covers marker parsing (signal → requester role) and click-zone
//! containment (column coord → click-hit). These are the testable
//! pure slices of the otherwise TUI-integrated feature.

use super::event::{RESTART_REQUEST_MARKER, parse_restart_request};

#[test]
fn parse_restart_request_extracts_role_after_colon() {
    let text = format!("{RESTART_REQUEST_MARKER}:dev");
    assert_eq!(parse_restart_request(&text), Some("dev".to_string()));
}

#[test]
fn parse_restart_request_defaults_to_agent_when_empty_role() {
    let text = RESTART_REQUEST_MARKER.to_string();
    assert_eq!(parse_restart_request(&text), Some("agent".to_string()));
}

#[test]
fn parse_restart_request_returns_none_for_unrelated_text() {
    assert_eq!(parse_restart_request("hello there"), None);
    assert_eq!(parse_restart_request(""), None);
    assert_eq!(parse_restart_request("almost_BPMUX_RESTART"), None);
}

#[test]
fn parse_restart_request_trims_role_whitespace() {
    let text = format!("{RESTART_REQUEST_MARKER}:  pm  ");
    assert_eq!(parse_restart_request(&text), Some("pm".to_string()));
}

#[test]
fn parse_restart_request_handles_any_role_name() {
    for role in &["dev", "pm", "designer", "ryan", "intern"] {
        let text = format!("{RESTART_REQUEST_MARKER}:{role}");
        assert_eq!(parse_restart_request(&text), Some((*role).to_string()));
    }
}

/// Click-zone containment check: a click at column `col` hits the
/// restart affordance iff the column falls within [start, end).
/// Kept as a pure helper mirror of the click handler logic in
/// event_dispatch.rs so the geometry is testable without a frame.
fn hits_restart_zone(col: u16, zone: (u16, u16)) -> bool {
    let (start, end) = zone;
    col >= start && col < end
}

#[test]
fn click_zone_hit_when_col_inside_range() {
    // Zone covers columns 10..20 (start=10, end=20)
    assert!(hits_restart_zone(10, (10, 20)));
    assert!(hits_restart_zone(15, (10, 20)));
    assert!(hits_restart_zone(19, (10, 20)));
}

#[test]
fn click_zone_miss_at_boundary_end() {
    // end is exclusive — col == end is a miss, hits the next zone
    assert!(!hits_restart_zone(20, (10, 20)));
}

#[test]
fn click_zone_miss_before_start() {
    assert!(!hits_restart_zone(9, (10, 20)));
    assert!(!hits_restart_zone(0, (10, 20)));
}

#[test]
fn click_zone_empty_range_matches_nothing() {
    // Degenerate zone: start == end — no columns hit
    assert!(!hits_restart_zone(10, (10, 10)));
    assert!(!hits_restart_zone(11, (10, 10)));
}
