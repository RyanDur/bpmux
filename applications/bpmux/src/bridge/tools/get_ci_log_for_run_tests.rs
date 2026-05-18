#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools::get_ci_log_for_run::parse_playwright_failures_from_log;

#[test]
fn placeholder_until_per_stage_parsers_added() {
    let report = parse_playwright_failures_from_log(0, "");
    assert_eq!(report.conclusion, "success");
}
