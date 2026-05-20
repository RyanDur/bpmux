#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::domain::{
    Story, StoryState, derive_epic_counts, derive_epic_state, derive_iteration_member_section,
    substitute_member_stories_section,
};
use chrono::{TimeZone, Utc};

#[test]
fn story_state_parse_round_trips_via_as_str() {
    for state in [
        StoryState::Icebox,
        StoryState::Ready,
        StoryState::InProgress,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::Acceptance,
        StoryState::Accepted,
    ] {
        let s = state.as_str();
        assert_eq!(StoryState::parse(s), Some(state), "parse({s:?}) round-trip");
    }
}

#[test]
fn story_state_parse_accepts_underscore_variants() {
    assert_eq!(
        StoryState::parse("in_progress"),
        Some(StoryState::InProgress)
    );
    assert_eq!(StoryState::parse("desk_check"), Some(StoryState::DeskCheck));
}

#[test]
fn story_state_parse_returns_none_for_unknown() {
    assert_eq!(StoryState::parse("not-a-state"), None);
    assert_eq!(StoryState::parse(""), None);
    assert_eq!(StoryState::parse("Ready"), None);
}

#[test]
fn story_state_acceptance_variant_parses_canonically() {
    assert_eq!(
        StoryState::parse("acceptance"),
        Some(StoryState::Acceptance)
    );
    assert_eq!(StoryState::Acceptance.as_str(), "acceptance");
}

#[test]
fn story_state_delivered_legacy_label_preserved_for_backward_compat() {
    assert_eq!(StoryState::parse("delivered"), Some(StoryState::Delivered));
    assert_eq!(StoryState::Delivered.as_str(), "delivered");
}

#[test]
fn story_state_github_label_canonical_form_per_variant() {
    assert_eq!(StoryState::Icebox.github_label(), "icebox");
    assert_eq!(StoryState::Ready.github_label(), "ready");
    assert_eq!(StoryState::InProgress.github_label(), "in progress");
    assert_eq!(StoryState::DeskCheck.github_label(), "desk-check");
    assert_eq!(StoryState::Delivered.github_label(), "delivered");
    assert_eq!(StoryState::Acceptance.github_label(), "acceptance");
    assert_eq!(StoryState::Accepted.github_label(), "accepted");
    assert_eq!(StoryState::Retired.github_label(), "retired");
}

#[test]
fn story_state_github_label_round_trips_via_state_from_labels() {
    for state in [
        StoryState::Ready,
        StoryState::InProgress,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::Accepted,
        StoryState::Retired,
    ] {
        let label = state.github_label();
        let parsed = crate::board::adapters::github::state_from_labels(&[label.to_string()], false);
        assert_eq!(parsed, state, "state_from_labels round-trip for {state:?}");
    }
}

#[test]
fn story_state_github_label_differs_from_as_str_on_in_progress_and_matches_otherwise() {
    assert_ne!(
        StoryState::InProgress.github_label(),
        StoryState::InProgress.as_str(),
        "InProgress is the only variant where github_label (spaces) diverges from as_str (kebab)"
    );
    for state in [
        StoryState::Icebox,
        StoryState::Ready,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::Acceptance,
        StoryState::Accepted,
        StoryState::Retired,
    ] {
        assert_eq!(
            state.github_label(),
            state.as_str(),
            "github_label and as_str agree for {state:?}"
        );
    }
}

#[test]
fn derive_member_section_returns_placeholder_when_empty_1462() {
    let section = derive_iteration_member_section(&[]);
    assert_eq!(section, "_(no members yet)_\n");
}

#[test]
fn derive_member_section_renders_single_member_as_bullet_1462() {
    let section = derive_iteration_member_section(&[(1309, "Add ci_get_job_status MCP tool")]);
    assert_eq!(section, "- #1309 — Add ci_get_job_status MCP tool\n");
}

#[test]
fn derive_member_section_renders_multi_member_in_input_order_1462() {
    let members = [
        (1309, "Add ci_get_job_status MCP tool"),
        (1265, "Reorder_story PM-only authority-gate"),
        (1462, "Iteration body auto-derive"),
    ];
    let section = derive_iteration_member_section(&members);
    assert_eq!(
        section,
        "- #1309 — Add ci_get_job_status MCP tool\n\
         - #1265 — Reorder_story PM-only authority-gate\n\
         - #1462 — Iteration body auto-derive\n"
    );
}

#[test]
fn substitute_member_section_replaces_placeholder_with_derived_list_1462() {
    let body = "## Iteration Telescopium\n\
                \n\
                **Created**: 2026-05-19\n\
                **Status**: Open\n\
                \n\
                ## Member stories\n\
                \n\
                <!-- Members carry `iteration-Telescopium.{position}` labels;\n\
                     ordering is by position ascending. -->\n\
                \n\
                _(no members yet)_\n\
                \n\
                ## Notes\n\
                \n\
                bpmux iteration substrate.\n";
    let derived = "- #1309 — Add ci_get_job_status MCP tool\n\
                   - #1265 — Reorder_story PM-only authority-gate\n";
    let out = substitute_member_stories_section(body, derived);
    assert!(
        out.contains("## Member stories\n"),
        "header preserved: {out}"
    );
    assert!(
        out.contains("- #1309 — Add ci_get_job_status MCP tool\n"),
        "first member appears: {out}"
    );
    assert!(
        out.contains("- #1265 — Reorder_story PM-only authority-gate\n"),
        "second member appears: {out}"
    );
    assert!(
        !out.contains("_(no members yet)_"),
        "placeholder replaced: {out}"
    );
    assert!(out.contains("## Notes\n"), "next H2 preserved: {out}");
    assert!(
        out.contains("bpmux iteration substrate."),
        "downstream Notes content preserved: {out}"
    );
}

#[test]
fn substitute_member_section_with_empty_derived_keeps_placeholder_form_1462() {
    let body = "## Iteration Telescopium\n\
                \n\
                ## Member stories\n\
                \n\
                _(no members yet)_\n\
                \n\
                ## Notes\n\
                bpmux.\n";
    let derived = derive_iteration_member_section(&[]);
    let out = substitute_member_stories_section(body, &derived);
    assert!(
        out.contains("_(no members yet)_"),
        "empty derivation surfaces placeholder: {out}"
    );
    assert!(out.contains("## Notes\n"), "next H2 preserved: {out}");
}

#[test]
fn substitute_member_section_returns_body_unchanged_when_header_missing_1462() {
    let body = "## Iteration No-Members-Section\n\n## Notes\nbpmux.\n";
    let derived = "- #1234 — some title\n";
    let out = substitute_member_stories_section(body, derived);
    assert_eq!(out, body, "no member-stories header → body unchanged");
}

#[test]
fn substitute_member_section_handles_body_without_trailing_h2_1462() {
    let body = "## Iteration Solo\n\n## Member stories\n\n_(no members yet)_\n";
    let derived = "- #42 — Solo member\n";
    let out = substitute_member_stories_section(body, derived);
    assert!(
        out.contains("- #42 — Solo member\n"),
        "derived list inserted when no trailing H2: {out}"
    );
    assert!(
        !out.contains("_(no members yet)_"),
        "placeholder replaced: {out}"
    );
}

#[test]
fn substitute_member_section_idempotent_under_re_derivation_1462() {
    let body = "## Iteration Pegasus\n\
                \n\
                ## Member stories\n\
                \n\
                _(no members yet)_\n\
                \n\
                ## Notes\n\
                end.\n";
    let derived = "- #100 — first\n- #200 — second\n";
    let once = substitute_member_stories_section(body, derived);
    let twice = substitute_member_stories_section(&once, derived);
    assert_eq!(
        once, twice,
        "re-deriving with same input is idempotent (Scenarios 3-4 reactive-on-read)"
    );
}

#[test]
fn substitute_member_section_re_derivation_reflects_membership_mutation_1462() {
    let body = "## Iteration Carina\n\
                \n\
                ## Member stories\n\
                \n\
                _(no members yet)_\n\
                \n\
                ## Notes\n\
                end.\n";
    let before = substitute_member_stories_section(body, "- #1 — a\n- #2 — b\n");
    let after_remove = substitute_member_stories_section(&before, "- #2 — b\n");
    assert!(
        !after_remove.contains("- #1 — a\n"),
        "removed story disappears on re-derivation: {after_remove}"
    );
    assert!(
        after_remove.contains("- #2 — b\n"),
        "remaining member preserved: {after_remove}"
    );
    let after_add = substitute_member_stories_section(&after_remove, "- #2 — b\n- #3 — c\n");
    assert!(
        after_add.contains("- #3 — c\n"),
        "added story appears on re-derivation: {after_add}"
    );
}

fn epic_child(number: u64, state: StoryState) -> Story {
    Story::new_minimal(
        number,
        format!("child-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    )
}

#[test]
fn derive_epic_state_returns_icebox_for_empty_children_1450() {
    assert_eq!(
        derive_epic_state(StoryState::Ready, &[]),
        StoryState::Icebox,
        "empty children must derive Icebox regardless of explicit state"
    );
}

#[test]
fn derive_epic_state_returns_icebox_when_all_children_icebox_1450() {
    let kids = [
        epic_child(1, StoryState::Icebox),
        epic_child(2, StoryState::Icebox),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Ready, &refs),
        StoryState::Icebox,
        "all-Icebox children derive Icebox"
    );
}

#[test]
fn derive_epic_state_returns_ready_for_one_ready_no_active_1450() {
    let kids = [
        epic_child(1, StoryState::Ready),
        epic_child(2, StoryState::Icebox),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Icebox, &refs),
        StoryState::Ready,
        "1+ Ready child + no active children derives Ready"
    );
}

#[test]
fn derive_epic_state_returns_in_progress_for_active_child_1450() {
    for active_state in [
        StoryState::InProgress,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::Acceptance,
    ] {
        let kids = [
            epic_child(1, StoryState::Ready),
            epic_child(2, active_state),
        ];
        let refs: Vec<&Story> = kids.iter().collect();
        assert_eq!(
            derive_epic_state(StoryState::Icebox, &refs),
            StoryState::InProgress,
            "active-state {active_state:?} child derives InProgress at Epic-axis"
        );
    }
}

#[test]
fn derive_epic_state_returns_accepted_when_all_non_retired_accepted_1450() {
    let kids = [
        epic_child(1, StoryState::Accepted),
        epic_child(2, StoryState::Accepted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Icebox, &refs),
        StoryState::Accepted,
        "all-Accepted children derive Accepted"
    );
}

#[test]
fn derive_epic_state_returns_retired_when_all_retired_1450() {
    let kids = [
        epic_child(1, StoryState::Retired),
        epic_child(2, StoryState::Retired),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Accepted, &refs),
        StoryState::Retired,
        "all-Retired children derive Retired (overrides explicit Accepted)"
    );
}

#[test]
fn derive_epic_state_mixed_accepted_and_retired_no_active_derives_accepted_1450() {
    let kids = [
        epic_child(1, StoryState::Accepted),
        epic_child(2, StoryState::Retired),
        epic_child(3, StoryState::Accepted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Icebox, &refs),
        StoryState::Accepted,
        "mixed Accepted + Retired (no active) derives Accepted — retired don't block accept"
    );
}

#[test]
fn derive_epic_state_ignores_explicit_state_when_children_present_1450() {
    let kids = [epic_child(1, StoryState::InProgress)];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Accepted, &refs),
        StoryState::InProgress,
        "post-1450: explicit_state is ignored — substrate IS source-of-truth (META-CANON #1)"
    );
    assert_eq!(
        derive_epic_state(StoryState::Retired, &refs),
        StoryState::InProgress,
        "Retired explicit_state also ignored when active children present"
    );
}

#[test]
fn derive_epic_state_active_dominates_terminal_at_in_progress_axis_1450() {
    let kids = [
        epic_child(1, StoryState::Accepted),
        epic_child(2, StoryState::Retired),
        epic_child(3, StoryState::InProgress),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Accepted, &refs),
        StoryState::InProgress,
        "1+ active child dominates Accepted/Retired siblings"
    );
}

#[test]
fn derive_epic_state_ready_with_terminal_children_derives_ready_1450() {
    let kids = [
        epic_child(1, StoryState::Ready),
        epic_child(2, StoryState::Accepted),
        epic_child(3, StoryState::Retired),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Icebox, &refs),
        StoryState::Ready,
        "Ready + terminal-only siblings (no active) derives Ready — terminal don't block Ready"
    );
}

#[test]
fn derive_epic_state_1189_paradox_regression_1450() {
    let mut kids: Vec<Story> = Vec::new();
    for n in 1..=30 {
        kids.push(epic_child(n, StoryState::Accepted));
    }
    for n in 31..=36 {
        kids.push(epic_child(n, StoryState::Retired));
    }
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(StoryState::Icebox, &refs),
        StoryState::Accepted,
        "#1189 paradox: Epic with 30 Accepted + 6 Retired children derives Accepted, NOT Icebox (Ryan-canon 2026-05-19 01:47 UTC)"
    );
}

#[test]
fn derive_epic_state_does_not_return_non_epic_states_1450() {
    let kids = [
        epic_child(1, StoryState::DeskCheck),
        epic_child(2, StoryState::Delivered),
        epic_child(3, StoryState::Acceptance),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let derived = derive_epic_state(StoryState::Icebox, &refs);
    assert!(
        matches!(
            derived,
            StoryState::Icebox
                | StoryState::Ready
                | StoryState::InProgress
                | StoryState::Accepted
                | StoryState::Retired
        ),
        "Epic state must be one of {{Icebox, Ready, InProgress, Accepted, Retired}}; never DeskCheck/Delivered/Acceptance; got {derived:?}"
    );
    assert_eq!(
        derived,
        StoryState::InProgress,
        "DeskCheck/Delivered/Acceptance children roll up to InProgress"
    );
}

#[test]
fn derive_epic_counts_empty_children_returns_zero_axes_1451() {
    let counts = derive_epic_counts(&[]);
    assert_eq!(counts.accepted_count, 0);
    assert_eq!(counts.retired_count, 0);
    assert_eq!(counts.active_count, 0);
    assert_eq!(counts.icebox_count, 0);
    assert_eq!(counts.total_count, 0);
    assert_eq!(counts.progress_denominator(), 0);
}

#[test]
fn derive_epic_counts_single_axis_per_state_1451() {
    let kids = [
        epic_child(1, StoryState::Accepted),
        epic_child(2, StoryState::Accepted),
        epic_child(3, StoryState::Retired),
        epic_child(4, StoryState::Ready),
        epic_child(5, StoryState::Icebox),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(counts.accepted_count, 2);
    assert_eq!(counts.retired_count, 1);
    assert_eq!(counts.active_count, 1, "Ready is in active-set");
    assert_eq!(counts.icebox_count, 1);
    assert_eq!(counts.total_count, 5);
}

#[test]
fn derive_epic_counts_active_includes_all_canonical_active_states_1451() {
    let kids = [
        epic_child(1, StoryState::Ready),
        epic_child(2, StoryState::InProgress),
        epic_child(3, StoryState::DeskCheck),
        epic_child(4, StoryState::Delivered),
        epic_child(5, StoryState::Acceptance),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(
        counts.active_count, 5,
        "active-set must include Ready+InProgress+DeskCheck+Delivered+Acceptance (sister to S1 derivation rule 2)"
    );
    assert_eq!(counts.accepted_count, 0);
    assert_eq!(counts.retired_count, 0);
    assert_eq!(counts.icebox_count, 0);
}

#[test]
fn derive_epic_counts_progress_denominator_excludes_only_retired_1451() {
    let kids = [
        epic_child(1, StoryState::Accepted),
        epic_child(2, StoryState::Accepted),
        epic_child(3, StoryState::Accepted),
        epic_child(4, StoryState::Retired),
        epic_child(5, StoryState::Retired),
        epic_child(6, StoryState::Ready),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(counts.accepted_count, 3);
    assert_eq!(counts.retired_count, 2);
    assert_eq!(counts.active_count, 1);
    assert_eq!(counts.icebox_count, 0);
    assert_eq!(counts.total_count, 6);
    assert_eq!(
        counts.progress_denominator(),
        4,
        "progress_denominator = accepted + active + icebox (only retired excluded as soft-delete); 3 + 1 + 0 = 4"
    );
}

#[test]
fn derive_epic_counts_progress_denominator_includes_icebox_1451_canon_refinement() {
    let kids = [
        epic_child(1, StoryState::Ready),
        epic_child(2, StoryState::Icebox),
        epic_child(3, StoryState::Icebox),
        epic_child(4, StoryState::Icebox),
        epic_child(5, StoryState::Icebox),
        epic_child(6, StoryState::Icebox),
        epic_child(7, StoryState::Icebox),
        epic_child(8, StoryState::Icebox),
        epic_child(9, StoryState::Icebox),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(counts.accepted_count, 0);
    assert_eq!(counts.active_count, 1);
    assert_eq!(counts.icebox_count, 8);
    assert_eq!(counts.retired_count, 0);
    assert_eq!(
        counts.progress_denominator(),
        9,
        "Epic #1477 canon-truth: 1 active + 8 icebox = 9 stories needing completion; icebox INCLUDED in denom (Ryan-canon 2026-05-20 23:12 UTC)"
    );
}

#[test]
fn derive_epic_counts_progress_denominator_mixed_excludes_retired_only_1451_canon() {
    let kids = [
        epic_child(1, StoryState::Accepted),
        epic_child(2, StoryState::Accepted),
        epic_child(3, StoryState::InProgress),
        epic_child(4, StoryState::Icebox),
        epic_child(5, StoryState::Icebox),
        epic_child(6, StoryState::Retired),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(counts.accepted_count, 2);
    assert_eq!(counts.active_count, 1);
    assert_eq!(counts.icebox_count, 2);
    assert_eq!(counts.retired_count, 1);
    assert_eq!(
        counts.progress_denominator(),
        5,
        "mixed: 2 accepted + 1 active + 2 icebox = 5 (retired excluded); fraction reads 2/5"
    );
}

#[test]
fn derive_epic_counts_1189_paradox_regression_1451() {
    let mut kids: Vec<Story> = Vec::new();
    for n in 1..=25 {
        kids.push(epic_child(n, StoryState::Accepted));
    }
    for n in 26..=30 {
        kids.push(epic_child(n, StoryState::Retired));
    }
    for n in 31..=36 {
        kids.push(epic_child(n, StoryState::InProgress));
    }
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(counts.accepted_count, 25, "#1189 truth: 25 shipped");
    assert_eq!(
        counts.retired_count, 5,
        "#1189 truth: 5 retired (scope-removed)"
    );
    assert_eq!(counts.active_count, 6, "#1189 truth: 6 active");
    assert_eq!(counts.total_count, 36);
    assert_eq!(
        counts.progress_denominator(),
        31,
        "true progress = 25/31 (not 30/36 conflated)"
    );
}

#[test]
fn derive_epic_counts_all_retired_zero_accepted_1451() {
    let kids = [
        epic_child(1, StoryState::Retired),
        epic_child(2, StoryState::Retired),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(
        counts.accepted_count, 0,
        "retired-children do NOT contribute to accepted-count (per `feedback_walk_surface_vs_conviction_converge_class_discrimination`)"
    );
    assert_eq!(counts.retired_count, 2);
    assert_eq!(counts.active_count, 0);
    assert_eq!(counts.total_count, 2);
    assert_eq!(
        counts.progress_denominator(),
        0,
        "all-retired Epic has no relevant-work — denominator is 0 (visually 0/0 + retired-axis surfaces the scope-removal)"
    );
}

#[test]
fn derive_epic_counts_icebox_separate_from_active_1451() {
    let kids = [
        epic_child(1, StoryState::Icebox),
        epic_child(2, StoryState::Icebox),
        epic_child(3, StoryState::Ready),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(counts.icebox_count, 2);
    assert_eq!(
        counts.active_count, 1,
        "Icebox NOT included in active-count"
    );
}

#[test]
fn derive_epic_counts_total_equals_sum_of_axes_1451() {
    let kids = [
        epic_child(1, StoryState::Accepted),
        epic_child(2, StoryState::Retired),
        epic_child(3, StoryState::Ready),
        epic_child(4, StoryState::InProgress),
        epic_child(5, StoryState::Icebox),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs);
    assert_eq!(
        counts.total_count,
        counts.accepted_count + counts.retired_count + counts.active_count + counts.icebox_count,
        "total_count is sum of 4 discriminated axes — no overlap, no gap"
    );
}
