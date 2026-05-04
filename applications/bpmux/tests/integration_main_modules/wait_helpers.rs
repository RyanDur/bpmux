//! Tests for the wait-for-condition helpers added in #198. These
//! exercise the harness primitives themselves — contract tests for the
//! test infrastructure, not a feature under test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use harness::pty::TestBpmux;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// AC: "Given a condition that is already met when checked,
/// when the wait-for-condition is called, then it returns immediately
/// with no delay."
#[test]
fn wait_until_returns_immediately_when_condition_already_true() {
    let bpmux = TestBpmux::start();
    let start = Instant::now();
    let result = bpmux.wait_until("always-true", Duration::from_secs(5), || true);
    let elapsed = start.elapsed();
    assert!(result.is_ok(), "wait_until should succeed");
    assert!(
        elapsed < Duration::from_millis(100),
        "should return immediately, took {elapsed:?}"
    );
}

/// AC: "Given a test that currently uses thread::sleep to wait for a
/// message to appear, when the test is rewritten with wait-for-condition,
/// then the test passes faster and more reliably."
#[test]
fn wait_until_succeeds_when_condition_becomes_true_within_timeout() {
    let bpmux = TestBpmux::start();
    let flag = Arc::new(AtomicBool::new(false));
    let flag_setter = Arc::clone(&flag);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        flag_setter.store(true, Ordering::SeqCst);
    });
    let start = Instant::now();
    let result = bpmux.wait_until("flag", Duration::from_secs(2), || {
        flag.load(Ordering::SeqCst)
    });
    let elapsed = start.elapsed();
    assert!(result.is_ok(), "wait_until should succeed: {result:?}");
    assert!(
        elapsed < Duration::from_millis(500),
        "should return shortly after flag flips, took {elapsed:?}"
    );
}

/// AC: "Given a condition that is never met, when the timeout expires,
/// then the test fails with a clear message showing what was expected
/// vs what was found."
#[test]
fn wait_until_fails_with_diagnostic_when_condition_never_true() {
    let bpmux = TestBpmux::start();
    let result = bpmux.wait_until("never-true", Duration::from_millis(200), || false);
    let err = result.expect_err("should fail on timeout");
    assert!(err.contains("timeout"), "err should mention timeout: {err}");
    assert!(
        err.contains("never-true"),
        "err should include the `what` label: {err}"
    );
}
