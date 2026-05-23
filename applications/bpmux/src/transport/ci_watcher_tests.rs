use super::ci_watcher::{Observation, Transition, classify_transition};

fn obs(run_id: u64, conclusion: &'static str) -> Observation {
    (Some(run_id), conclusion)
}

#[test]
fn first_observation_is_a_change() {
    let t = classify_transition(None, obs(1, "success"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

#[test]
fn same_run_and_same_conclusion_is_no_broadcast() {
    let t = classify_transition(Some(obs(42, "success")), obs(42, "success"));
    assert_eq!(t, Transition::Same);
}

#[test]
fn in_progress_to_success_same_run_is_a_change() {
    let t = classify_transition(Some(obs(42, "in_progress")), obs(42, "success"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

#[test]
fn success_to_failure_different_run_is_a_change() {
    let t = classify_transition(Some(obs(42, "success")), obs(43, "failure"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "failure"
        }
    );
}

#[test]
fn failure_to_in_progress_different_run_is_a_change() {
    let t = classify_transition(Some(obs(42, "failure")), obs(43, "in_progress"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "in_progress"
        }
    );
}

#[test]
fn unknown_to_any_real_state_same_run_is_a_change() {
    let t = classify_transition(Some(obs(42, "unknown")), obs(42, "success"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

#[test]
fn new_run_same_conclusion_is_a_change_not_same() {
    let t = classify_transition(Some(obs(42, "failure")), obs(43, "failure"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "failure"
        },
        "new run_id with same conclusion must broadcast — pre-#395 \
         bug was that successive reds silently fell through"
    );
}

#[test]
fn same_run_same_conclusion_dedups() {
    let t = classify_transition(Some(obs(99, "failure")), obs(99, "failure"));
    assert_eq!(
        t,
        Transition::Same,
        "identical (run_id, conclusion) must dedup — otherwise \
         every tick re-emits and the channel fills with noise"
    );
}

#[test]
fn conclusion_flip_same_run_preserves_existing_broadcast_behavior() {
    let t = classify_transition(Some(obs(7, "in_progress")), obs(7, "failure"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "failure"
        }
    );
}

#[test]
fn run_id_none_degrades_to_conclusion_only_dedup() {
    let t = classify_transition(Some((None, "success")), (None, "success"));
    assert_eq!(t, Transition::Same);
}

mod backoff_decisions {
    use crate::github::{
        BackoffSignal, CLOCK_SKEW_GRACE_SECS, ObservedWakes, RateLimitObserver, decide_backoff,
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn at_epoch(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn no_headers_preserves_exponential_floor() {
        let now = at_epoch(1_700_000_000);
        let decision = decide_backoff(ObservedWakes::default(), Duration::from_secs(120), now);
        assert_eq!(decision.signal, BackoffSignal::Exponential);
        assert_eq!(decision.wait, Duration::from_secs(120));
    }

    #[test]
    fn retry_after_dominates_when_higher_than_exponential() {
        let now = at_epoch(1_700_000_000);
        let observed = ObservedWakes {
            retry_after_wake_epoch: 1_700_000_300,
            ratelimit_reset_epoch: 0,
        };
        let decision = decide_backoff(observed, Duration::from_secs(60), now);
        assert_eq!(decision.signal, BackoffSignal::RetryAfterHeader);
        assert_eq!(decision.wait, Duration::from_secs(300));
    }

    #[test]
    fn exponential_dominates_when_higher_than_retry_after() {
        let now = at_epoch(1_700_000_000);
        let observed = ObservedWakes {
            retry_after_wake_epoch: 1_700_000_030,
            ratelimit_reset_epoch: 0,
        };
        let decision = decide_backoff(observed, Duration::from_secs(1800), now);
        assert_eq!(decision.signal, BackoffSignal::Exponential);
        assert_eq!(decision.wait, Duration::from_secs(1800));
    }

    #[test]
    fn ratelimit_reset_includes_clock_skew_grace() {
        let now = at_epoch(1_700_000_000);
        let observed = ObservedWakes {
            retry_after_wake_epoch: 0,
            ratelimit_reset_epoch: 1_700_000_600,
        };
        let decision = decide_backoff(observed, Duration::from_secs(60), now);
        assert_eq!(decision.signal, BackoffSignal::RatelimitResetHeader);
        assert_eq!(
            decision.wait,
            Duration::from_secs(600 + CLOCK_SKEW_GRACE_SECS)
        );
    }

    #[test]
    fn both_headers_present_max_wins() {
        let now = at_epoch(1_700_000_000);
        let observed = ObservedWakes {
            retry_after_wake_epoch: 1_700_000_300,
            ratelimit_reset_epoch: 1_700_000_900,
        };
        let decision = decide_backoff(observed, Duration::from_secs(60), now);
        assert_eq!(decision.signal, BackoffSignal::RatelimitResetHeader);
        assert_eq!(
            decision.wait,
            Duration::from_secs(900 + CLOCK_SKEW_GRACE_SECS)
        );
    }

    #[test]
    fn observer_records_retry_after_and_decides_correctly() {
        let observer = RateLimitObserver::new();
        let now = at_epoch(1_700_000_000);
        observer.observe_retry_after_secs(45, now);
        let decision = decide_backoff(observer.snapshot(), Duration::from_secs(10), now);
        assert_eq!(decision.signal, BackoffSignal::RetryAfterHeader);
        assert_eq!(decision.wait, Duration::from_secs(45));
    }

    #[test]
    fn ratelimit_reset_in_past_yields_zero_wait() {
        let now = at_epoch(1_700_000_500);
        let observed = ObservedWakes {
            retry_after_wake_epoch: 0,
            ratelimit_reset_epoch: 1_700_000_000,
        };
        let decision = decide_backoff(observed, Duration::from_secs(0), now);
        assert_eq!(decision.wait, Duration::from_secs(0));
    }
}

mod cascade_integration {
    use crate::github::{
        BackoffSignal, RateLimitObserver, classify_and_decide, is_rate_limit_error,
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn at_epoch(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn rate_limited_error_consults_observer_snapshot() {
        let observer = RateLimitObserver::new();
        let now = at_epoch(1_700_000_000);
        observer.observe_retry_after_secs(300, now);
        let result = classify_and_decide(
            "rate limited — retrying",
            observer.snapshot(),
            Duration::from_secs(60),
            now,
        );
        assert_eq!(
            result,
            Some(crate::github::BackoffDecision {
                wait: Duration::from_secs(300),
                signal: BackoffSignal::RetryAfterHeader,
            })
        );
    }

    #[test]
    fn non_rate_limit_error_returns_none() {
        let observer = RateLimitObserver::new();
        let now = at_epoch(1_700_000_000);
        observer.observe_retry_after_secs(300, now);
        let result = classify_and_decide(
            "network unreachable",
            observer.snapshot(),
            Duration::from_secs(60),
            now,
        );
        assert!(result.is_none());
    }

    #[test]
    fn rate_limit_string_variants_all_classify_as_rate_limited() {
        assert!(is_rate_limit_error("rate limited — retrying"));
        assert!(is_rate_limit_error("Rate Limit exceeded"));
        assert!(is_rate_limit_error("api rate-limit hit"));
        assert!(is_rate_limit_error("RATE LIMITED"));
        assert!(!is_rate_limit_error("offline"));
        assert!(!is_rate_limit_error("auth invalid"));
        assert!(!is_rate_limit_error("github error 500"));
    }

    #[test]
    fn cascade_falls_back_to_exponential_when_observer_empty() {
        let observer = RateLimitObserver::new();
        let now = at_epoch(1_700_000_000);
        let result = classify_and_decide(
            "rate limited",
            observer.snapshot(),
            Duration::from_secs(120),
            now,
        );
        assert_eq!(
            result,
            Some(crate::github::BackoffDecision {
                wait: Duration::from_secs(120),
                signal: BackoffSignal::Exponential,
            })
        );
    }

    #[test]
    fn cascade_uses_max_when_observer_lower_than_exponential() {
        let observer = RateLimitObserver::new();
        let now = at_epoch(1_700_000_000);
        observer.observe_retry_after_secs(30, now);
        let result = classify_and_decide(
            "rate limited",
            observer.snapshot(),
            Duration::from_secs(1800),
            now,
        );
        assert_eq!(
            result,
            Some(crate::github::BackoffDecision {
                wait: Duration::from_secs(1800),
                signal: BackoffSignal::Exponential,
            })
        );
    }
}
