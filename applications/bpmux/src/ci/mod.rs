#[cfg(test)]
pub(crate) use ::ci::CiStateReport;
pub use ::ci::{
    CiEvent, CiJobRecord, CiJobStatusError, CiLogError, CiLogStore, CiRunFailures, CiState,
    CiStateStore, CiTestFailure, ConditionalRunEntry, GITHUB_BASE_URL, MAX_BACKOFF, POLL_INTERVAL,
    RunEntry, WorkflowRunEntry, WorkflowSummary, build_non_test_failure_record, classify_runs,
    compute_duration_ms, github_actions_url, parse_playwright_failures_from_log,
};
pub(crate) use ::ci::{
    build_dev_ci_signal, build_multi_depth_for_transition, compute_broadcast_text,
    unknown_ci_state_report,
};
pub mod slice;
