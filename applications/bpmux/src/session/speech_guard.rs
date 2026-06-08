const BANNED: [&str; 8] = [
    "standing by",
    "monitoring",
    "active-trigger-watch",
    "presence-mode",
    "watching for",
    "hold position",
    "passive watch",
    "passive-watch",
];

const NAMED_CAUSE: [&str; 13] = [
    "queue-empty",
    "blocker",
    "owner-signal",
    "red-ci",
    "red ci",
    "next-pull",
    "substrate-bound",
    "legitimate-deferral",
    "named-active-trigger",
    "named-empty",
    "awaiting-owner",
    "ci-red",
    "test-failure",
];

pub fn pause_vocab_violation(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let stripped = strip_quoted(&lower);
    let matched = BANNED
        .iter()
        .filter_map(|banned| stripped.find(banned).map(|pos| (pos, *banned)))
        .min_by_key(|(pos, _)| *pos)
        .map(|(_, banned)| banned)?;
    if NAMED_CAUSE.iter().any(|cause| lower.contains(cause)) {
        return None;
    }
    let mut without_banned = stripped;
    for banned in BANNED {
        without_banned = without_banned.replace(banned, "");
    }
    if has_for_noun_clause(&without_banned) {
        return None;
    }
    Some(reason(matched))
}

fn reason(matched: &str) -> String {
    format!(
        "self-imposed-pause vocabulary detected (\"{matched}\") without a named stop-cause. \
         Per the board-speech-guard (#493): a broadcast that signals pause/standby requires a \
         NAMED cause (queue-empty, blocker, owner-signal, red-CI, next-pull, substrate-bound, \
         legitimate-deferral, named-active-trigger). Either rewrite to explicit active work, or \
         name the stop-cause explicitly."
    )
}

fn strip_quoted(text: &str) -> String {
    let mut out = text.to_string();
    for quote in ['\'', '"', '`'] {
        out = strip_spans(&out, quote);
    }
    out
}

fn strip_spans(text: &str, quote: char) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == quote
            && let Some(close) = (i + 1..chars.len()).find(|&j| chars[j] == quote)
        {
            i = close + 1;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn has_for_noun_clause(text: &str) -> bool {
    for (idx, _) in text.match_indices(" for ") {
        let rest = &text[idx + " for ".len()..];
        let mut chars = rest.chars();
        let first = chars.next();
        let second = chars.next();
        if matches!(first, Some(c) if c.is_ascii_lowercase())
            && matches!(second, Some(c) if c.is_ascii_lowercase() || c == '-')
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

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
}
