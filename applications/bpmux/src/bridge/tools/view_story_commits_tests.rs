#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view_story_commits::{parse_git_log_output, subject_matches_story};

#[test]
fn subject_match_accepts_trailing_colon() {
    assert!(subject_matches_story("#411: see commits", 411));
}

#[test]
fn subject_match_accepts_trailing_space() {
    assert!(subject_matches_story("#411 rework: helper", 411));
}

#[test]
fn subject_match_rejects_prefix_of_larger_number() {
    assert!(!subject_matches_story("#4110: unrelated", 411));
}

#[test]
fn subject_match_rejects_substring_within_number() {
    assert!(!subject_matches_story("#14113 adjacent", 411));
}

#[test]
fn subject_match_accepts_mid_subject() {
    assert!(subject_matches_story("rework fix for #411 issue", 411));
}

#[test]
fn subject_match_accepts_trailing_end() {
    assert!(subject_matches_story("fixes #411", 411));
}

#[test]
fn subject_match_rejects_missing_hash() {
    assert!(!subject_matches_story("411 not a story reference", 411));
}

#[test]
fn parse_returns_empty_when_no_matches() {
    let text = "a\x1fb\x1funrelated subject\x1fdev\x1f2026-04-23T00:00:00Z\n";
    assert!(parse_git_log_output(text, 411).is_empty());
}

#[test]
fn parse_returns_matching_commit_with_all_fields() {
    let text = "abcdef1234\x1fabcdef1\x1f#411: land feature\x1fdev\x1f2026-04-24T00:00:00Z\n";
    let commits = parse_git_log_output(text, 411);
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0]["sha"], "abcdef1234");
    assert_eq!(commits[0]["short_sha"], "abcdef1");
    assert_eq!(commits[0]["subject"], "#411: land feature");
    assert_eq!(commits[0]["author"], "dev");
    assert_eq!(commits[0]["committed_at"], "2026-04-24T00:00:00Z");
}

#[test]
fn parse_surfaces_multiple_commits_landing_same_story() {
    let text = "\
aaa\x1faaa\x1f#411: feature\x1fdev\x1f2026-04-20T00:00:00Z
bbb\x1fbbb\x1f#411 rework: helper\x1fdev\x1f2026-04-21T00:00:00Z
ccc\x1fccc\x1f#411 rework fix: bug\x1fdev\x1f2026-04-22T00:00:00Z
";
    let commits = parse_git_log_output(text, 411);
    assert_eq!(commits.len(), 3);
    let subjects: Vec<&str> = commits
        .iter()
        .map(|c| c["subject"].as_str().unwrap())
        .collect();
    assert!(subjects.contains(&"#411: feature"));
    assert!(subjects.contains(&"#411 rework: helper"));
    assert!(subjects.contains(&"#411 rework fix: bug"));
}

#[test]
fn parse_filters_out_subject_that_mentions_but_doesnt_land_story() {
    let text = "\
aaa\x1faaa\x1f#411: lands the feature\x1fdev\x1f2026-04-20T00:00:00Z
bbb\x1fbbb\x1f#412: follows up but does not land 411\x1fdev\x1f2026-04-21T00:00:00Z
";
    let commits = parse_git_log_output(text, 411);
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0]["subject"], "#411: lands the feature");
}

#[test]
fn parse_skips_malformed_lines() {
    let text = "\
abc\x1fdef\x1fmissing-fields
aaa\x1faaa\x1f#411: valid\x1fdev\x1f2026-04-20T00:00:00Z
";
    let commits = parse_git_log_output(text, 411);
    assert_eq!(commits.len(), 1);
}

#[test]
fn parse_disambiguates_4110_from_411() {
    let text = "\
aaa\x1faaa\x1f#4110: different story\x1fdev\x1f2026-04-20T00:00:00Z
bbb\x1fbbb\x1f#411: our story\x1fdev\x1f2026-04-21T00:00:00Z
";
    let commits = parse_git_log_output(text, 411);
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0]["subject"], "#411: our story");
}
