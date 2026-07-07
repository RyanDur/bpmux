#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::deliver_story_body_diff_gate::*;
use crate::product_management::story::{CastVerdict, Story, StoryId, Verdict, Work};
use chrono::{DateTime, TimeZone, Utc};

fn story_with_pickup(pickup_ts: DateTime<Utc>) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(1001),
        "test".to_string(),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap(),
        "dev".to_string(),
    );
    s.verdicts = Some(vec![CastVerdict {
        verdict: Verdict::Succeed {
            step: "ready".into(),
        },
        ts: pickup_ts,
        actor: "dev".to_string(),
        payload: Some("pickup paraphrase".to_string()),
    }]);
    s
}

fn fake_git_returning(lines: &[&str]) -> impl Fn(&[&str]) -> Result<String, String> {
    let owned: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    move |_args| Ok(owned.join("\n"))
}

fn fake_git_error() -> impl Fn(&[&str]) -> Result<String, String> {
    |_args| Err("git unavailable".to_string())
}

#[test]
fn pickup_ts_picks_most_recent_pickup_gate() {
    let early = Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap();
    let late = Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap();
    let mut s = story_with_pickup(early);
    if let Some(gc) = s.verdicts.as_mut() {
        gc.push(CastVerdict {
            verdict: Verdict::Succeed {
                step: "ready".into(),
            },
            ts: late,
            actor: "dev".to_string(),
            payload: None,
        });
    }
    assert_eq!(pickup_ts(&s), Some(late));
}

#[test]
fn pickup_ts_returns_none_when_no_verdicts() {
    let s = Story::new_minimal(
        StoryId::new(1),
        "t".to_string(),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap(),
        "dev".to_string(),
    );
    assert!(pickup_ts(&s).is_none());
}

#[test]
fn changed_paths_since_parses_git_output_lines() {
    let ts = Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap();
    let git = fake_git_returning(&[
        "applications/bpmux/src/bridge/deliver_story_body_diff_gate.rs",
        "",
        "applications/bpmux/src/bridge/atp_substantive_gate.rs",
    ]);
    let paths = changed_paths_since(ts, &git).unwrap();
    assert_eq!(
        paths,
        vec![
            "applications/bpmux/src/bridge/deliver_story_body_diff_gate.rs",
            "applications/bpmux/src/bridge/atp_substantive_gate.rs"
        ]
    );
}

#[test]
fn verify_body_atp_in_changed_paths_passes_when_atp_in_changed() {
    let body = "## Acceptance test path\n`applications/foo.test.tsx`";
    let changed = vec!["applications/foo.test.tsx".to_string()];
    assert!(verify_body_atp_in_changed_paths(body, &changed).is_ok());
}

#[test]
fn verify_body_atp_in_changed_paths_blocks_when_atp_missing() {
    let body = "## Acceptance test path\n`applications/missing.test.tsx`";
    let changed = vec!["applications/other.test.tsx".to_string()];
    let err = verify_body_atp_in_changed_paths(body, &changed).unwrap_err();
    assert!(err.contains("body-diff-vs-impl gate blocks"));
    assert!(err.contains("applications/missing.test.tsx"));
}

#[test]
fn verify_body_atp_in_changed_paths_lenient_when_body_has_no_source_atp() {
    let body = "## Acceptance test path\n`docs/audit-logs/2026-05-16.md`";
    let changed: Vec<String> = vec![];
    assert!(verify_body_atp_in_changed_paths(body, &changed).is_ok());
}

#[test]
fn verify_deliver_drift_passes_when_pickup_ts_missing() {
    let body = "## Acceptance test path\n`applications/never.test.tsx`";
    let s = Story::new_minimal(
        StoryId::new(1),
        "t".to_string(),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap(),
        "dev".to_string(),
    );
    let git = fake_git_returning(&[]);
    assert!(verify_deliver_drift(body, &s, &git).is_ok());
}

#[test]
fn verify_deliver_drift_passes_when_git_errors() {
    let body = "## Acceptance test path\n`applications/foo.test.tsx`";
    let s = story_with_pickup(Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap());
    let git = fake_git_error();
    assert!(verify_deliver_drift(body, &s, &git).is_ok());
}

#[test]
fn verify_deliver_drift_blocks_when_body_atp_missing_from_changed() {
    let body = "## Acceptance test path\n`applications/foo.test.tsx`";
    let s = story_with_pickup(Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap());
    let git = fake_git_returning(&["applications/bar.test.tsx"]);
    let err = verify_deliver_drift(body, &s, &git).unwrap_err();
    assert!(err.contains("body-diff-vs-impl gate blocks"));
}

#[test]
fn verify_deliver_drift_passes_when_body_atp_in_changed() {
    let body = "## Acceptance test path\n`applications/foo.test.tsx`";
    let s = story_with_pickup(Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap());
    let git = fake_git_returning(&["applications/foo.test.tsx"]);
    assert!(verify_deliver_drift(body, &s, &git).is_ok());
}

#[test]
fn verify_deliver_drift_lenient_when_no_commits_since_pickup() {
    let body = "## Acceptance test path\n`applications/foo.test.tsx`";
    let s = story_with_pickup(Utc.with_ymd_and_hms(2026, 5, 16, 22, 0, 0).unwrap());
    let git = fake_git_returning(&[]);
    assert!(
        verify_deliver_drift(body, &s, &git).is_ok(),
        "empty changed-paths means no commits since pickup; can't meaningfully diff body-ATP"
    );
}
