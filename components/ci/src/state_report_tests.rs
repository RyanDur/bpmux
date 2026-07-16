#![allow(clippy::unwrap_used)]

use super::state_report::*;
use crate::{MAX_BACKOFF, POLL_INTERVAL};

#[test]
fn unknown_report_carries_the_poll_interval_and_backoff_cap_contract_bounds() {
    let unknown = unknown_ci_state_report();
    assert_eq!(unknown.poll_interval_secs, POLL_INTERVAL.as_secs());
    assert_eq!(unknown.backoff_cap_secs, MAX_BACKOFF.as_secs());
}
