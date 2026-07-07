#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::{Decision, advance};
use crate::product_management::story::{Stage, Verdict, Work};
use crate::workflow::bundled_default::bundled_default;
use crate::workflow::test_support::repo_workflow;

fn succeed(step: &str) -> Verdict {
    Verdict::Succeed { step: step.into() }
}

fn fail(step: &str) -> Verdict {
    Verdict::Fail { step: step.into() }
}

#[test]
fn succeed_from_ready_advances_to_in_progress_entry() {
    let wf = repo_workflow();
    assert_eq!(
        advance(&succeed("ready"), &wf),
        Some(Decision {
            coarse: Work::InProgress,
            placement: None
        })
    );
}

#[test]
fn succeed_from_in_progress_advances_to_desk_check_placement() {
    let wf = repo_workflow();
    assert_eq!(
        advance(&succeed("in-progress"), &wf),
        Some(Decision {
            coarse: Work::InProgress,
            placement: Some(Stage::DeskCheck.id())
        })
    );
}

#[test]
fn succeed_from_desk_check_crosses_to_acceptance() {
    let wf = repo_workflow();
    assert_eq!(
        advance(&succeed("desk-check"), &wf),
        Some(Decision {
            coarse: Work::Acceptance,
            placement: None
        })
    );
}

#[test]
fn succeed_from_acceptance_crosses_to_accepted() {
    let wf = repo_workflow();
    assert_eq!(
        advance(&succeed("acceptance"), &wf),
        Some(Decision {
            coarse: Work::Accepted,
            placement: None
        })
    );
}

#[test]
fn succeed_from_the_bundled_single_middle_crosses_straight_to_acceptance() {
    let wf = bundled_default();
    assert_eq!(
        advance(&succeed("in-progress"), &wf),
        Some(Decision {
            coarse: Work::Acceptance,
            placement: None
        })
    );
}

#[test]
fn fail_from_desk_check_derives_return_to_in_progress_entry() {
    let wf = repo_workflow();
    assert_eq!(
        advance(&fail("desk-check"), &wf),
        Some(Decision {
            coarse: Work::InProgress,
            placement: None
        })
    );
}

#[test]
fn fail_from_acceptance_derives_return_to_ready() {
    let wf = repo_workflow();
    assert_eq!(
        advance(&fail("acceptance"), &wf),
        Some(Decision {
            coarse: Work::NotStarted,
            placement: None
        })
    );
}

#[test]
fn succeed_from_unknown_state_yields_none() {
    let wf = repo_workflow();
    assert_eq!(advance(&succeed("nonexistent-state"), &wf), None);
}

#[test]
fn fail_from_state_without_a_backward_transition_yields_none() {
    let wf = repo_workflow();
    assert_eq!(advance(&fail("ready"), &wf), None);
}

fn three_stage_workflow(review_fail_to: Option<&str>) -> crate::workflow::WorkflowConfig {
    use crate::workflow::{Section, State, WorkflowConfig};
    let state = |id: &str, section: &str, fail_to: Option<&str>| State {
        id: id.into(),
        section: section.into(),
        judges: vec![],
        fail_to: fail_to.map(str::to_string),
    };
    WorkflowConfig {
        role: vec![],
        section: vec![
            Section {
                id: "pre-work".into(),
            },
            Section {
                id: "in-work".into(),
            },
            Section {
                id: "post-work".into(),
            },
        ],
        state: vec![
            state("icebox", "pre-work", None),
            state("ready", "pre-work", None),
            state("build", "in-work", None),
            state("review", "in-work", review_fail_to),
            state("harden", "in-work", None),
            state("acceptance", "post-work", None),
            state("accepted", "post-work", None),
            state("retired", "post-work", None),
        ],
        transition: vec![],
    }
}

#[test]
fn fail_at_an_unconfigured_middle_stage_defaults_to_the_previous_stage() {
    let wf = three_stage_workflow(None);
    assert_eq!(
        advance(&fail("harden"), &wf),
        Some(Decision {
            coarse: Work::InProgress,
            placement: Some(crate::workflow::StageId::from("review"))
        }),
        "an unconfigured failed judgment sends the story back one stage"
    );
    assert_eq!(
        advance(&fail("review"), &wf),
        Some(Decision {
            coarse: Work::InProgress,
            placement: None
        }),
        "back one stage from the second stage is the middle's entry"
    );
}

#[test]
fn fail_at_the_first_middle_stage_stays_in_that_stage() {
    let wf = three_stage_workflow(None);
    assert_eq!(
        advance(&fail("build"), &wf),
        Some(Decision {
            coarse: Work::InProgress,
            placement: None
        }),
        "a first-stage failure reworks in place — leaving the middle is never a fail DEFAULT"
    );
}

#[test]
fn configured_fail_to_wins_over_the_default_ladder() {
    let wf = three_stage_workflow(Some("ready"));
    assert_eq!(
        advance(&fail("review"), &wf),
        Some(Decision {
            coarse: Work::NotStarted,
            placement: None
        }),
        "fail_to is the team's explicit call and beats every default"
    );
}

#[test]
fn fail_at_an_unconfigured_post_work_state_still_yields_none() {
    let wf = three_stage_workflow(None);
    assert_eq!(
        advance(&fail("acceptance"), &wf),
        None,
        "the default ladder is for middle-stage judgments; an unconfigured acceptance fail stays loud"
    );
}
