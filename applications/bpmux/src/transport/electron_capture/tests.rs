#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use chrono::TimeZone;
use std::path::Path;

#[test]
fn build_capture_filename_uses_iso_compact_timestamp_and_role_slug() {
    let now = chrono::Utc.with_ymd_and_hms(2026, 5, 5, 1, 30, 45).unwrap();
    let name = build_capture_filename("dev", now);
    assert_eq!(name, "20260505T013045Z-dev.png");
}

#[test]
fn build_capture_filename_slugs_unsafe_chars() {
    let now = chrono::Utc.with_ymd_and_hms(2026, 5, 5, 1, 30, 45).unwrap();
    let name = build_capture_filename("agent/with spaces", now);
    assert_eq!(name, "20260505T013045Z-agent_with_spaces.png");
}

#[test]
fn build_capture_filename_falls_back_to_agent_when_role_is_only_special_chars() {
    let now = chrono::Utc.with_ymd_and_hms(2026, 5, 5, 1, 30, 45).unwrap();
    let name = build_capture_filename("///", now);
    assert!(
        name.contains("-agent.png"),
        "all-special role slugs must fall back to 'agent' to avoid trailing dash"
    );
}

#[test]
fn cleanup_old_captures_returns_zero_for_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let removed = cleanup_old_captures(tmp.path());
    assert_eq!(removed, 0);
}

#[test]
fn cleanup_old_captures_leaves_fresh_files_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let fresh = tmp.path().join("fresh.png");
    std::fs::write(&fresh, b"x").unwrap();
    let removed = cleanup_old_captures(tmp.path());
    assert_eq!(removed, 0, "fresh files (mtime ~now) must be retained");
    assert!(fresh.exists());
}

#[test]
fn cleanup_old_captures_silently_handles_missing_dir() {
    let removed = cleanup_old_captures(Path::new("/nonexistent/captures/dir"));
    assert_eq!(removed, 0, "missing dir is not an error — return 0");
}

#[test]
fn capture_error_serializes_with_named_kind_per_falsifier_a_and_j() {
    let err = CaptureError::ElectronNotRunning;
    let s = serde_json::to_string(&err).unwrap();
    assert!(s.contains("\"kind\":\"electron_not_running\""), "got: {s}");
    let err = CaptureError::WindowMinimized;
    let s = serde_json::to_string(&err).unwrap();
    assert!(s.contains("\"kind\":\"window_minimized\""), "got: {s}");
    let err = CaptureError::NoWindow;
    let s = serde_json::to_string(&err).unwrap();
    assert!(s.contains("\"kind\":\"no_window\""), "got: {s}");
}

#[test]
fn parse_window_discovery_output_extracts_wid_width_height_for_visible_window() {
    let parsed = parse_window_discovery_output("12345 1280 800").unwrap();
    assert_eq!(
        parsed,
        DiscoveredWindow {
            window_id: 12345,
            width: 1280,
            height: 800,
        }
    );
}

#[test]
fn parse_window_discovery_output_no_window_marker_returns_no_window_error() {
    let err = parse_window_discovery_output("NO_WINDOW").unwrap_err();
    assert_eq!(err, CaptureError::NoWindow);
}

#[test]
fn parse_window_discovery_output_no_electron_marker_returns_electron_not_running() {
    let err = parse_window_discovery_output("NO_ELECTRON").unwrap_err();
    assert_eq!(err, CaptureError::ElectronNotRunning);
}

#[test]
fn parse_window_discovery_output_zero_dimensions_means_window_minimized() {
    let err = parse_window_discovery_output("12345 0 0").unwrap_err();
    assert_eq!(err, CaptureError::WindowMinimized);
}

#[test]
fn parse_window_discovery_output_takes_last_line_so_swift_warnings_do_not_break_it() {
    let parsed = parse_window_discovery_output("warning: deprecated API\n12345 1280 800").unwrap();
    assert_eq!(parsed.window_id, 12345);
}

#[test]
fn classify_screencapture_failure_recognizes_screen_recording_perm_denial() {
    let err = classify_screencapture_failure("could not create image from window", Some(1));
    assert!(matches!(err, CaptureError::PermissionDenied { .. }));
    let err = classify_screencapture_failure("Not authorized to capture", Some(1));
    assert!(matches!(err, CaptureError::PermissionDenied { .. }));
}

#[test]
fn classify_screencapture_failure_falls_back_to_capture_failed_for_other_errors() {
    let err = classify_screencapture_failure("disk full", Some(1));
    assert!(
        matches!(err, CaptureError::CaptureFailed { .. }),
        "non-permission errors must map to capture_failed, not permission_denied"
    );
}

#[test]
fn permission_denied_serializes_with_grant_instructions_in_detail() {
    let err = CaptureError::PermissionDenied {
        detail: "Grant Screen Recording permission".to_string(),
    };
    let s = serde_json::to_string(&err).unwrap();
    assert!(s.contains("\"kind\":\"permission_denied\""), "got: {s}");
    assert!(
        s.contains("Screen Recording"),
        "agent must see grant-path in detail; got: {s}"
    );
}
