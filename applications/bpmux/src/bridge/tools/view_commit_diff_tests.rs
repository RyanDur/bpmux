#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view_commit_diff::{is_safe_ref, truncate_diff};

#[test]
fn safe_ref_accepts_full_sha() {
    assert!(is_safe_ref("abcdef1234567890abcdef1234567890abcdef12"));
}

#[test]
fn safe_ref_accepts_short_sha() {
    assert!(is_safe_ref("abc1234"));
}

#[test]
fn safe_ref_accepts_branch_name() {
    assert!(is_safe_ref("main"));
}

#[test]
fn safe_ref_accepts_tag_with_dots() {
    assert!(is_safe_ref("v1.2.3"));
}

#[test]
fn safe_ref_accepts_path_style_ref() {
    assert!(is_safe_ref("origin/main"));
}

#[test]
fn safe_ref_rejects_empty() {
    assert!(!is_safe_ref(""));
}

#[test]
fn safe_ref_rejects_spaces() {
    assert!(!is_safe_ref("abc def"));
}

#[test]
fn safe_ref_rejects_shell_metacharacters() {
    assert!(!is_safe_ref("abc; rm -rf /"));
    assert!(!is_safe_ref("abc|cat"));
    assert!(!is_safe_ref("abc`whoami`"));
    assert!(!is_safe_ref("abc$(ls)"));
    assert!(!is_safe_ref("abc&&echo"));
}

#[test]
fn safe_ref_rejects_newlines() {
    assert!(!is_safe_ref("abc\nwhoami"));
}

#[test]
fn truncate_passes_through_when_under_cap() {
    let diff = "short diff content";
    assert_eq!(truncate_diff(diff, 1000), diff);
}

#[test]
fn truncate_exactly_at_cap_passes_through() {
    let diff = "12345";
    assert_eq!(truncate_diff(diff, 5), diff);
}

#[test]
fn truncate_appends_marker_when_over_cap() {
    let diff = "1234567890";
    let result = truncate_diff(diff, 4);
    assert!(
        result.starts_with("1234"),
        "shown prefix should be first 4 bytes: {result}"
    );
    assert!(
        result.contains("truncated"),
        "marker should mention truncation: {result}"
    );
    assert!(
        result.contains("10 bytes"),
        "marker should cite total: {result}"
    );
}

#[test]
fn truncate_marker_uses_safe_char_boundary_for_utf8() {
    let diff = "héllo wörld lots of content";
    let result = truncate_diff(diff, 2);
    assert!(
        result.starts_with("h") && !result.starts_with("hé"),
        "should step down to 1-byte boundary when cap=2 falls mid-é: {result}"
    );
}

#[test]
fn truncate_reports_shown_bytes_count_accurately() {
    let diff = "12345678901234567890";
    let result = truncate_diff(diff, 10);
    assert!(
        result.contains("10 of 20"),
        "should cite shown of total: {result}"
    );
}
