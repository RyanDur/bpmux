//! Unit tests for the pure `resolve_diag_path` router. Drives every
//! precedence branch from #512: BPMUX_LOG override, BPMUX_TEST_MODE,
//! cargo-test exe-path detection, prod-default, no-home fallback.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::diag::{path_has_deps_segment, resolve_diag_path};
use std::path::{Path, PathBuf};

fn home_dir() -> PathBuf {
    PathBuf::from("/fake/home")
}

fn tmp_dir() -> PathBuf {
    PathBuf::from("/fake/tmp")
}

#[test]
fn bpmux_log_override_wins_in_prod_mode() {
    let resolved = resolve_diag_path(
        Some("/custom/diag.log"),
        None,
        false,
        Some(&home_dir()),
        &tmp_dir(),
    );
    assert_eq!(resolved, Some(PathBuf::from("/custom/diag.log")));
}

#[test]
fn bpmux_log_override_wins_even_in_test_mode() {
    let resolved = resolve_diag_path(
        Some("/custom/diag.log"),
        Some("1"),
        true,
        Some(&home_dir()),
        &tmp_dir(),
    );
    assert_eq!(resolved, Some(PathBuf::from("/custom/diag.log")));
}

#[test]
fn empty_bpmux_log_falls_through_to_test_mode() {
    let resolved = resolve_diag_path(Some(""), Some("1"), false, Some(&home_dir()), &tmp_dir());
    assert_eq!(resolved, Some(tmp_dir().join("bpmux-test-diag.log")));
}

#[test]
fn bpmux_test_mode_routes_to_tmpdir() {
    let resolved = resolve_diag_path(None, Some("1"), false, Some(&home_dir()), &tmp_dir());
    assert_eq!(resolved, Some(tmp_dir().join("bpmux-test-diag.log")));
}

#[test]
fn cargo_test_exe_routes_to_tmpdir_without_env_var() {
    let resolved = resolve_diag_path(None, None, true, Some(&home_dir()), &tmp_dir());
    assert_eq!(resolved, Some(tmp_dir().join("bpmux-test-diag.log")));
}

#[test]
fn empty_bpmux_test_mode_still_falls_back_to_exe_detection() {
    let resolved = resolve_diag_path(None, Some(""), true, Some(&home_dir()), &tmp_dir());
    assert_eq!(resolved, Some(tmp_dir().join("bpmux-test-diag.log")));
}

#[test]
fn prod_mode_routes_to_home_bpmux_diag() {
    let resolved = resolve_diag_path(None, None, false, Some(&home_dir()), &tmp_dir());
    assert_eq!(resolved, Some(PathBuf::from("/fake/home/.bpmux/diag.log")));
}

#[test]
fn no_home_no_test_mode_returns_none() {
    let resolved = resolve_diag_path(None, None, false, None, &tmp_dir());
    assert_eq!(resolved, None);
}

#[test]
fn no_home_in_test_mode_still_returns_tmp_path() {
    let resolved = resolve_diag_path(None, Some("1"), false, None, &tmp_dir());
    assert_eq!(resolved, Some(tmp_dir().join("bpmux-test-diag.log")));
}

#[test]
fn deps_segment_detected_in_cargo_test_layout() {
    assert!(path_has_deps_segment(PathBuf::from(
        "/repo/target/debug/deps/bpmux-abc123"
    )));
    assert!(path_has_deps_segment(PathBuf::from(
        "/repo/target/release/deps/integration_test-xyz"
    )));
}

#[test]
fn deps_segment_absent_in_production_binary_layout() {
    assert!(!path_has_deps_segment(PathBuf::from(
        "/repo/target/debug/bpmux"
    )));
    assert!(!path_has_deps_segment(PathBuf::from(
        "/usr/local/bin/bpmux"
    )));
    assert!(!path_has_deps_segment(PathBuf::from(
        "/repo/target/release/bpmux"
    )));
}

#[test]
fn deps_segment_must_be_full_component_not_substring() {
    assert!(!path_has_deps_segment(PathBuf::from(
        "/repo/target/debug/depsfake/bin"
    )));
    assert!(!path_has_deps_segment(PathBuf::from("/foo/bar-deps/baz")));
}

#[test]
fn home_bpmux_diag_for_path_segments() {
    let resolved = resolve_diag_path(
        None,
        None,
        false,
        Some(Path::new("/users/test")),
        &tmp_dir(),
    );
    assert_eq!(resolved, Some(PathBuf::from("/users/test/.bpmux/diag.log")));
}
