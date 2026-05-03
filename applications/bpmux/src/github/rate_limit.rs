use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackoffSignal {
    RetryAfterHeader,
    RatelimitResetHeader,
    Exponential,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackoffDecision {
    pub wait: Duration,
    pub signal: BackoffSignal,
}

pub const CLOCK_SKEW_GRACE_SECS: u64 = 5;

#[derive(Clone, Default)]
pub struct RateLimitObserver {
    inner: Arc<State>,
}

#[derive(Default)]
struct State {
    retry_after_wake_epoch: AtomicU64,
    ratelimit_reset_epoch: AtomicU64,
}

impl RateLimitObserver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_retry_after_secs(&self, secs: u64, now: SystemTime) {
        let now_epoch = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.inner
            .retry_after_wake_epoch
            .store(now_epoch.saturating_add(secs), Ordering::Relaxed);
    }

    pub fn observe_ratelimit_reset_epoch(&self, epoch: u64) {
        self.inner
            .ratelimit_reset_epoch
            .store(epoch, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> ObservedWakes {
        ObservedWakes {
            retry_after_wake_epoch: self.inner.retry_after_wake_epoch.load(Ordering::Relaxed),
            ratelimit_reset_epoch: self.inner.ratelimit_reset_epoch.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ObservedWakes {
    pub retry_after_wake_epoch: u64,
    pub ratelimit_reset_epoch: u64,
}

pub fn decide_backoff(
    observed: ObservedWakes,
    exponential: Duration,
    now: SystemTime,
) -> BackoffDecision {
    let now_epoch = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let exponential_wake_epoch = now_epoch.saturating_add(exponential.as_secs());

    let retry_after_wake = observed.retry_after_wake_epoch;
    let reset_wake = if observed.ratelimit_reset_epoch == 0 {
        0
    } else {
        observed
            .ratelimit_reset_epoch
            .saturating_add(CLOCK_SKEW_GRACE_SECS)
    };

    let mut chosen = exponential_wake_epoch;
    let mut signal = BackoffSignal::Exponential;
    if retry_after_wake > chosen {
        chosen = retry_after_wake;
        signal = BackoffSignal::RetryAfterHeader;
    }
    if reset_wake > chosen {
        chosen = reset_wake;
        signal = BackoffSignal::RatelimitResetHeader;
    }

    let wait_secs = chosen.saturating_sub(now_epoch);
    BackoffDecision {
        wait: Duration::from_secs(wait_secs),
        signal,
    }
}

pub fn capture_headers(headers: &http::HeaderMap, observer: &RateLimitObserver, now: SystemTime) {
    if let Some(secs) = headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
    {
        observer.observe_retry_after_secs(secs, now);
    }
    if let Some(epoch) = headers
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
    {
        observer.observe_ratelimit_reset_epoch(epoch);
    }
}

pub fn signal_label(signal: BackoffSignal) -> &'static str {
    match signal {
        BackoffSignal::RetryAfterHeader => "retry-after-header",
        BackoffSignal::RatelimitResetHeader => "x-ratelimit-reset-header",
        BackoffSignal::Exponential => "exponential",
    }
}
