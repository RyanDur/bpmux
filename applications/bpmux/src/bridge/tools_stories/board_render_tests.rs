#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use super::board_render::*;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::BoardState;
use crate::product_management::story::Story;
use crate::product_management::story::StoryId;
use crate::view::board::arrangement::Column;
use crate::workflow::StageId;
use chrono::{TimeZone, Utc};

fn bundled_middle() -> Vec<StageId> {
    crate::workflow::board_middle_stage_ids(&crate::workflow::test_support::repo_workflow())
}

fn story(num: u64, title: &str, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(num),
        title.to_string(),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn fine(placed: &[(u64, &str)]) -> HashMap<StoryId, StageId> {
    placed
        .iter()
        .map(|(id, stage)| (StoryId::new(*id), StageId::from(*stage)))
        .collect()
}

fn iteration_with(id: u64, story_ids: &[u64]) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: story_ids.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: false,
        complete: false,
    }
}

fn current_iteration_with(id: u64, story_ids: &[u64]) -> Vec<Iteration> {
    let mut iteration = iteration_with(id, story_ids);
    iteration.is_current = true;
    vec![iteration]
}

#[test]
fn a_config_defined_third_middle_stage_renders_as_its_own_column_in_order() {
    let middle = vec![
        StageId::from("in-progress"),
        StageId::from("code-review"),
        StageId::from("desk-check"),
    ];
    let stories = vec![story(42, "under review", BoardState::InProgress)];
    let out = build_board_text(
        &[],
        &stories,
        &[],
        &BoardScope::default(),
        &fine(&[(42, "code-review")]),
        &middle,
    );
    assert!(
        out.contains("#42 under review"),
        "a story in a config-defined middle stage appears on the board; got:\n{out}"
    );
    let in_progress = out.find("IN PROGRESS").expect("in-progress column");
    let code_review = out
        .find("CODE REVIEW")
        .expect("a config-defined middle stage renders its own column header");
    let desk_check = out.find("DESK CHECK").expect("desk-check column");
    assert!(
        in_progress < code_review && code_review < desk_check,
        "middle columns render in config order (in-progress, code-review, desk-check); got:\n{out}"
    );
    assert!(
        out[code_review..desk_check].contains("#42 under review"),
        "the story renders under its own config-defined column, not another; got:\n{out}"
    );
}

#[test]
fn default_scope_is_unchanged_full_board() {
    let stories = vec![
        story(1, "ready one", BoardState::NotStarted),
        story(2, "in-progress one", BoardState::InProgress),
    ];
    let out = build_board_text(
        &[],
        &stories,
        &[],
        &BoardScope::default(),
        &fine(&[]),
        &bundled_middle(),
    );
    assert!(out.contains("#1 ready one"));
    assert!(out.contains("#2 in-progress one"));
    assert!(out.contains("READY"));
    assert!(out.contains("IN PROGRESS"));
}

#[test]
fn a_card_surfaces_its_acceptor_so_the_team_sees_who_accepts() {
    let mut s = story(77, "acceptor-visible", BoardState::NotStarted);
    s.acceptor = Some(crate::product_management::story::Acceptor::Owner);
    let stories = [s];
    let out = build_board_text(
        &[],
        &stories,
        &[],
        &BoardScope::default(),
        &fine(&[]),
        &bundled_middle(),
    );
    assert!(
        out.contains("#77 acceptor-visible · awaiting owner"),
        "the board card surfaces the acceptor (instead of the team reading None) so the acceptor is visible before desk-check; got:\n{out}"
    );
}

#[test]
fn a_card_without_an_acceptor_renders_no_acceptor_segment() {
    let stories = [story(78, "no-acceptor-yet", BoardState::NotStarted)];
    let out = build_board_text(
        &[],
        &stories,
        &[],
        &BoardScope::default(),
        &fine(&[]),
        &bundled_middle(),
    );
    assert!(
        out.contains("#78 no-acceptor-yet") && !out.contains("#78 no-acceptor-yet · awaiting"),
        "an unset acceptor adds no segment — only present acceptors render; got:\n{out}"
    );
}

#[test]
fn state_scope_renders_only_the_matching_bucket() {
    let stories = vec![
        story(1, "ready one", BoardState::NotStarted),
        story(2, "in-progress one", BoardState::InProgress),
    ];
    let scope = BoardScope {
        state: Some(Column::Ready),
        ..BoardScope::default()
    };
    let out = build_board_text(
        &[],
        &stories,
        &current_iteration_with(500, &[1]),
        &scope,
        &fine(&[]),
        &bundled_middle(),
    );
    assert!(out.contains("#1 ready one"));
    assert!(!out.contains("#2 in-progress one"));
    assert!(!out.contains("IN PROGRESS"));
}

#[test]
fn iteration_scope_excludes_other_iterations() {
    let stories = vec![
        story(10, "iter-one story", BoardState::NotStarted),
        story(20, "iter-two story", BoardState::NotStarted),
    ];
    let iterations = vec![iteration_with(1, &[10]), iteration_with(2, &[20])];
    let scope = BoardScope {
        iteration: Some(1),
        ..BoardScope::default()
    };
    let out = build_board_text(
        &[],
        &stories,
        &iterations,
        &scope,
        &fine(&[]),
        &bundled_middle(),
    );
    assert!(out.contains("#10 iter-one story"));
    assert!(!out.contains("#20 iter-two story"));
}

#[test]
fn iteration_scope_derives_membership_from_the_container_not_the_story() {
    let stories = vec![story(30, "container-membered", BoardState::NotStarted)];
    let iterations = vec![iteration_with(5, &[30])];
    let scope = BoardScope {
        iteration: Some(5),
        ..BoardScope::default()
    };
    let out = build_board_text(
        &[],
        &stories,
        &iterations,
        &scope,
        &fine(&[]),
        &bundled_middle(),
    );
    assert!(
        out.contains("#30 container-membered"),
        "membership is the iteration container's concern (Iteration.stories); a story with no story-side field is still in scope when the iteration lists it; got:\n{out}"
    );
}

#[test]
fn row_limit_caps_entries_but_count_reflects_full_total() {
    let stories: Vec<Story> = (0..10)
        .map(|i| story(100 + i, &format!("ready {i}"), BoardState::NotStarted))
        .collect();
    let members: Vec<u64> = (0..10).map(|i| 100 + i).collect();
    let scope = BoardScope {
        state: Some(Column::Ready),
        row_limit: Some(3),
        ..BoardScope::default()
    };
    let out = build_board_text(
        &[],
        &stories,
        &current_iteration_with(500, &members),
        &scope,
        &fine(&[]),
        &bundled_middle(),
    );
    assert_eq!(out.matches("ready ").count(), 3);
    assert!(out.contains("(10)"));
}

#[test]
fn large_board_scoped_is_ingestible_while_full_is_not() {
    let stories: Vec<Story> = (0..4000u64)
            .map(|i| {
                let title = format!(
                    "story {i} with a sufficiently long descriptive title to inflate the full board payload"
                );
                let state = if i % 2 == 0 {
                    BoardState::NotStarted
                } else {
                    BoardState::Accepted
                };
                story(1000 + i, &title, state)
            })
            .collect();
    let full = build_board_text(
        &[],
        &stories,
        &[],
        &BoardScope::default(),
        &fine(&[]),
        &bundled_middle(),
    );
    let scoped = build_board_text(
        &[],
        &stories,
        &[],
        &BoardScope {
            state: Some(Column::Ready),
            row_limit: Some(20),
            ..BoardScope::default()
        },
        &fine(&[]),
        &bundled_middle(),
    );
    assert!(
        full.len() > 100_000,
        "full board should be large: {}",
        full.len()
    );
    assert!(
        scoped.len() < 10_000,
        "scoped board should be ingestible-whole: {}",
        scoped.len()
    );
    assert!(!scoped.contains("ACCEPTED"));
}
