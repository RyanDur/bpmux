#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::speech_guard::*;

fn blocked(text: &str) -> bool {
    pause_vocab_violation(text).is_some()
}

#[test]
fn active_standing_by_without_cause_is_blocked() {
    assert!(blocked("standing by"));
}

#[test]
fn active_monitoring_without_cause_is_blocked() {
    assert!(blocked("monitoring the channel"));
}

#[test]
fn active_hold_position_without_cause_is_blocked() {
    assert!(blocked("hold position"));
}

#[test]
fn prefix_pm_standing_by_without_cause_is_blocked() {
    assert!(blocked("PM standing by"));
}

#[test]
fn prefix_monitoring_team_channel_without_cause_is_blocked() {
    assert!(blocked("monitoring the team channel"));
}

#[test]
fn watching_for_changes_has_no_separate_cause_clause_and_is_blocked() {
    assert!(blocked("watching for changes"));
}

#[test]
fn standing_by_with_named_blocker_is_allowed() {
    assert!(!blocked("standing by — blocker on CI red-ci"));
}

#[test]
fn monitoring_with_queue_empty_is_allowed() {
    assert!(!blocked("monitoring; queue-empty"));
}

#[test]
fn hold_position_with_owner_signal_is_allowed() {
    assert!(!blocked("hold position — owner-signal pending"));
}

#[test]
fn single_quoted_self_correction_is_allowed() {
    assert!(!blocked(
        "PM used 'standing by' earlier — apologies; reshaping"
    ));
}

#[test]
fn double_quoted_narrative_is_allowed() {
    assert!(!blocked(
        "dev quoted \"standing by\" in narrative; not an active pause"
    ));
}

#[test]
fn backtick_quoted_correction_is_allowed() {
    assert!(!blocked("I used `standing by` last broadcast — fixed"));
}

#[test]
fn canon_explainer_quoting_the_phrase_is_allowed() {
    assert!(!blocked(
        "the pause-vocab hook fires on 'standing by' without named cause"
    ));
}

#[test]
fn named_cause_inside_a_quote_still_allows_per_documented_design() {
    assert!(!blocked(
        "standing by since 'blocker' was named in a prior note"
    ));
}

#[test]
fn no_banned_phrase_is_allowed() {
    assert!(!blocked("moving on to the next story"));
}

#[test]
fn empty_text_is_allowed() {
    assert!(!blocked(""));
}

#[test]
fn standing_by_for_ci_green_cause_clause_is_allowed() {
    assert!(!blocked("standing by for CI green on 80dea156"));
}

#[test]
fn standing_by_for_acceptance_call_cause_clause_is_allowed() {
    assert!(!blocked("PM standing by for your acceptance-call"));
}

#[test]
fn monitoring_for_owner_review_cause_clause_is_allowed() {
    assert!(!blocked("monitoring for owner review on the diff"));
}

#[test]
fn hold_position_for_pm_signal_cause_clause_is_allowed() {
    assert!(!blocked("hold position for pm-signal on next story"));
}

#[test]
fn reason_names_the_matched_phrase() {
    let reason = pause_vocab_violation("standing by").expect("blocked");
    assert!(reason.contains("standing by"));
    assert!(reason.contains("named stop-cause"));
}
