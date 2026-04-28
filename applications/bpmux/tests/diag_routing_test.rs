//! Integration test for #512: cargo test runs must NOT pollute the
//! production diag log at `~/.bpmux/diag.log`. Drives the live
//! `log_diag` entry-point, then asserts (1) a unique marker landed in
//! the test-isolated path under `${TMPDIR}` and (2) the production
//! path is either absent or untouched by the marker.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bpmux::session::diag::log_diag;
use std::fs;

#[test]
fn log_diag_under_cargo_test_writes_to_tmpdir_not_home() {
    let marker = format!(
        "DIAG-ROUTING-MARKER-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );

    let prod_path = std::env::var_os("HOME").map(|h| {
        let mut p = std::path::PathBuf::from(h);
        p.push(".bpmux");
        p.push("diag.log");
        p
    });
    let prod_before = prod_path.as_ref().and_then(|p| fs::read_to_string(p).ok());

    log_diag(&marker);

    let test_path = std::env::temp_dir().join("bpmux-test-diag.log");
    let test_after = fs::read_to_string(&test_path).expect("test diag log readable post-write");
    assert!(
        test_after.contains(&marker),
        "marker {marker} must land in {test_path:?}; got tail: {:?}",
        test_after.lines().last()
    );

    if let Some(p) = prod_path.as_ref() {
        let prod_after = fs::read_to_string(p).ok();
        match (prod_before, prod_after) {
            (Some(before), Some(after)) => assert_eq!(
                before, after,
                "production diag log at {p:?} must be untouched by cargo-test log_diag"
            ),
            (None, Some(after)) => assert!(
                !after.contains(&marker),
                "production diag log at {p:?} must not contain test marker {marker}"
            ),
            _ => {}
        }
    }
}
