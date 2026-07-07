#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::product_management::epic::{derive_epic_counts, derive_epic_state};
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::scheduling::ProjectedState;
use crate::product_management::story::{Story, StoryId, Work};
use chrono::{TimeZone, Utc};

fn epic_child(id: u64, state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("child-{id}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    )
}

fn current_iteration_of(members: &[u64]) -> Vec<Iteration> {
    vec![Iteration {
        id: IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: members.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }]
}

#[test]
fn derive_epic_state_returns_icebox_for_empty_children_1450() {
    assert_eq!(
        derive_epic_state(Work::NotStarted, &[], &[]),
        ProjectedState::Icebox,
        "empty children must derive Icebox regardless of explicit state"
    );
}

#[test]
fn derive_epic_state_returns_icebox_when_all_children_icebox_1450() {
    let kids = [
        epic_child(1, Work::NotStarted),
        epic_child(2, Work::NotStarted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::NotStarted, &refs, &[]),
        ProjectedState::Icebox,
        "all-Icebox children derive Icebox"
    );
}

#[test]
fn derive_epic_state_returns_ready_for_one_ready_no_active_1450() {
    let kids = [
        epic_child(1, Work::NotStarted),
        epic_child(2, Work::NotStarted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::NotStarted, &refs, &current_iteration_of(&[1])),
        ProjectedState::Ready,
        "1+ Ready child + no active children derives Ready"
    );
}

#[test]
fn derive_epic_state_returns_in_progress_for_active_child_1450() {
    for active_state in [Work::InProgress, Work::InProgress, Work::Acceptance] {
        let kids = [
            epic_child(1, Work::NotStarted),
            epic_child(2, active_state.clone()),
        ];
        let refs: Vec<&Story> = kids.iter().collect();
        assert_eq!(
            derive_epic_state(Work::NotStarted, &refs, &current_iteration_of(&[1])),
            ProjectedState::InProgress,
            "active-state {active_state:?} child derives InProgress at Epic-axis"
        );
    }
}

#[test]
fn derive_epic_state_returns_accepted_when_all_non_retired_accepted_1450() {
    let kids = [epic_child(1, Work::Accepted), epic_child(2, Work::Accepted)];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::NotStarted, &refs, &[]),
        ProjectedState::Accepted,
        "all-Accepted children derive Accepted"
    );
}

#[test]
fn derive_epic_state_returns_retired_when_all_retired_1450() {
    let kids = [epic_child(1, Work::Retired), epic_child(2, Work::Retired)];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::Accepted, &refs, &[]),
        ProjectedState::Retired,
        "all-Retired children derive Retired (overrides explicit Accepted)"
    );
}

#[test]
fn derive_epic_state_mixed_accepted_and_retired_no_active_derives_accepted_1450() {
    let kids = [
        epic_child(1, Work::Accepted),
        epic_child(2, Work::Retired),
        epic_child(3, Work::Accepted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::NotStarted, &refs, &[]),
        ProjectedState::Accepted,
        "mixed Accepted + Retired (no active) derives Accepted — retired don't block accept"
    );
}

#[test]
fn derive_epic_state_ignores_explicit_state_when_children_present_1450() {
    let kids = [epic_child(1, Work::InProgress)];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::Accepted, &refs, &[]),
        ProjectedState::InProgress,
        "post-1450: explicit_state is ignored — substrate IS source-of-truth (META-CANON #1)"
    );
    assert_eq!(
        derive_epic_state(Work::Retired, &refs, &[]),
        ProjectedState::InProgress,
        "Retired explicit_state also ignored when active children present"
    );
}

#[test]
fn derive_epic_state_active_dominates_terminal_at_in_progress_axis_1450() {
    let kids = [
        epic_child(1, Work::Accepted),
        epic_child(2, Work::Retired),
        epic_child(3, Work::InProgress),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::Accepted, &refs, &[]),
        ProjectedState::InProgress,
        "1+ active child dominates Accepted/Retired siblings"
    );
}

#[test]
fn derive_epic_state_not_started_non_member_child_derives_icebox() {
    let kids = [
        epic_child(1, Work::NotStarted),
        epic_child(2, Work::Accepted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::NotStarted, &refs, &current_iteration_of(&[])),
        ProjectedState::Icebox,
        "a not-started child outside the current iteration projects Icebox, so the epic's only non-terminal child cannot make it Ready"
    );
}

#[test]
fn derive_epic_state_ready_with_terminal_children_derives_ready_1450() {
    let kids = [
        epic_child(1, Work::NotStarted),
        epic_child(2, Work::Accepted),
        epic_child(3, Work::Retired),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::NotStarted, &refs, &current_iteration_of(&[1])),
        ProjectedState::Ready,
        "Ready + terminal-only siblings (no active) derives Ready — terminal don't block Ready"
    );
}

#[test]
fn derive_epic_state_1189_paradox_regression_1450() {
    let mut kids: Vec<Story> = Vec::new();
    for n in 1..=30 {
        kids.push(epic_child(n, Work::Accepted));
    }
    for n in 31..=36 {
        kids.push(epic_child(n, Work::Retired));
    }
    let refs: Vec<&Story> = kids.iter().collect();
    assert_eq!(
        derive_epic_state(Work::NotStarted, &refs, &[]),
        ProjectedState::Accepted,
        "#1189 paradox: Epic with 30 Accepted + 6 Retired children derives Accepted, NOT Icebox (owner-canon 2026-05-19 01:47 UTC)"
    );
}

#[test]
fn derive_epic_state_does_not_return_non_epic_states_1450() {
    let kids = [
        epic_child(1, Work::InProgress),
        epic_child(2, Work::Acceptance),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let derived = derive_epic_state(Work::NotStarted, &refs, &[]);
    assert!(
        matches!(
            derived,
            ProjectedState::Icebox
                | ProjectedState::Ready
                | ProjectedState::InProgress
                | ProjectedState::Accepted
                | ProjectedState::Retired
        ),
        "Epic state must be one of {{Icebox, Ready, InProgress, Accepted, Retired}}; never DeskCheck/Delivered; got {derived:?}"
    );
    assert_eq!(
        derived,
        ProjectedState::InProgress,
        "DeskCheck/Delivered children roll up to InProgress"
    );
}

#[test]
fn derive_epic_counts_empty_children_returns_zero_axes_1451() {
    let counts = derive_epic_counts(&[], &[]);
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
        epic_child(1, Work::Accepted),
        epic_child(2, Work::Accepted),
        epic_child(3, Work::Retired),
        epic_child(4, Work::NotStarted),
        epic_child(5, Work::NotStarted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &current_iteration_of(&[4]));
    assert_eq!(counts.accepted_count, 2);
    assert_eq!(counts.retired_count, 1);
    assert_eq!(counts.active_count, 1, "Ready is in active-set");
    assert_eq!(counts.icebox_count, 1);
    assert_eq!(counts.total_count, 5);
}

#[test]
fn derive_epic_counts_active_includes_all_canonical_active_states_1451() {
    let kids = [
        epic_child(1, Work::NotStarted),
        epic_child(2, Work::InProgress),
        epic_child(3, Work::InProgress),
        epic_child(4, Work::Acceptance),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &current_iteration_of(&[1]));
    assert_eq!(
        counts.active_count, 4,
        "active-set must include Ready+InProgress+DeskCheck+Delivered (sister to S1 derivation rule 2)"
    );
    assert_eq!(counts.accepted_count, 0);
    assert_eq!(counts.retired_count, 0);
    assert_eq!(counts.icebox_count, 0);
}

#[test]
fn derive_epic_counts_progress_denominator_excludes_only_retired_1451() {
    let kids = [
        epic_child(1, Work::Accepted),
        epic_child(2, Work::Accepted),
        epic_child(3, Work::Accepted),
        epic_child(4, Work::Retired),
        epic_child(5, Work::Retired),
        epic_child(6, Work::NotStarted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &current_iteration_of(&[6]));
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
        epic_child(1, Work::NotStarted),
        epic_child(2, Work::NotStarted),
        epic_child(3, Work::NotStarted),
        epic_child(4, Work::NotStarted),
        epic_child(5, Work::NotStarted),
        epic_child(6, Work::NotStarted),
        epic_child(7, Work::NotStarted),
        epic_child(8, Work::NotStarted),
        epic_child(9, Work::NotStarted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &current_iteration_of(&[1]));
    assert_eq!(counts.accepted_count, 0);
    assert_eq!(counts.active_count, 1);
    assert_eq!(counts.icebox_count, 8);
    assert_eq!(counts.retired_count, 0);
    assert_eq!(
        counts.progress_denominator(),
        9,
        "Epic #1477 canon-truth: 1 active + 8 icebox = 9 stories needing completion; icebox INCLUDED in denom (owner-canon 2026-05-20 23:12 UTC)"
    );
}

#[test]
fn derive_epic_counts_progress_denominator_mixed_excludes_retired_only_1451_canon() {
    let kids = [
        epic_child(1, Work::Accepted),
        epic_child(2, Work::Accepted),
        epic_child(3, Work::InProgress),
        epic_child(4, Work::NotStarted),
        epic_child(5, Work::NotStarted),
        epic_child(6, Work::Retired),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &[]);
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
        kids.push(epic_child(n, Work::Accepted));
    }
    for n in 26..=30 {
        kids.push(epic_child(n, Work::Retired));
    }
    for n in 31..=36 {
        kids.push(epic_child(n, Work::InProgress));
    }
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &[]);
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
    let kids = [epic_child(1, Work::Retired), epic_child(2, Work::Retired)];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &[]);
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
        epic_child(1, Work::NotStarted),
        epic_child(2, Work::NotStarted),
        epic_child(3, Work::NotStarted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &current_iteration_of(&[3]));
    assert_eq!(counts.icebox_count, 2);
    assert_eq!(
        counts.active_count, 1,
        "Icebox NOT included in active-count"
    );
}

#[test]
fn derive_epic_counts_total_equals_sum_of_axes_1451() {
    let kids = [
        epic_child(1, Work::Accepted),
        epic_child(2, Work::Retired),
        epic_child(3, Work::NotStarted),
        epic_child(4, Work::InProgress),
        epic_child(5, Work::NotStarted),
    ];
    let refs: Vec<&Story> = kids.iter().collect();
    let counts = derive_epic_counts(&refs, &current_iteration_of(&[3]));
    assert_eq!(
        counts.total_count,
        counts.accepted_count + counts.retired_count + counts.active_count + counts.icebox_count,
        "total_count is sum of 4 discriminated axes — no overlap, no gap"
    );
}
