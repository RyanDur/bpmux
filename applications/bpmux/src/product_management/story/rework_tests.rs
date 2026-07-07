#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::rework::most_recent_needs_work;
use super::{CastVerdict, Verdict};
use chrono::{TimeZone, Utc};

fn reject_cast_at(hour: u32, reason: &str) -> CastVerdict {
    CastVerdict {
        verdict: Verdict::Fail {
            step: "acceptance".into(),
        },
        ts: Utc.with_ymd_and_hms(2026, 4, 1, hour, 0, 0).unwrap(),
        actor: "owner".into(),
        payload: Some(reason.to_string()),
    }
}

fn accept_cast_at(hour: u32) -> CastVerdict {
    CastVerdict {
        verdict: Verdict::Succeed {
            step: "acceptance".into(),
        },
        ts: Utc.with_ymd_and_hms(2026, 4, 1, hour, 0, 0).unwrap(),
        actor: "owner".into(),
        payload: None,
    }
}

#[test]
fn the_latest_cast_reject_decides_rework_regardless_of_arrival_order() {
    let arrival_inverts_cast_order = [reject_cast_at(2, "narrow viewport"), accept_cast_at(1)];
    let rework = most_recent_needs_work(&arrival_inverts_cast_order);
    assert_eq!(
        rework.and_then(|v| v.payload.as_deref()),
        Some("narrow viewport"),
        "cast time is the product truth — a reject cast at T2 stays the active rework even when an earlier-cast entry arrives after it in the store list"
    );
}

#[test]
fn a_latest_cast_success_means_no_rework_in_either_arrival_order() {
    let cast_order = [reject_cast_at(1, "superseded"), accept_cast_at(2)];
    assert_eq!(
        most_recent_needs_work(&cast_order).map(|v| &v.verdict),
        None,
        "the latest cast is a success — the earlier reject is superseded"
    );
    let inverted_arrival = [accept_cast_at(2), reject_cast_at(1, "superseded")];
    assert_eq!(
        most_recent_needs_work(&inverted_arrival).map(|v| &v.verdict),
        None,
        "arrival order cannot resurrect a superseded reject"
    );
}

#[test]
fn no_verdicts_means_no_rework() {
    assert!(most_recent_needs_work(&[]).is_none());
}
