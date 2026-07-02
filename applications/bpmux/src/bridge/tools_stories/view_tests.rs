#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::view::*;
use crate::product_management::board::arrangement::ColumnBucket;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::BoardState;
use crate::product_management::story::Stage;
use crate::product_management::story::Story;
use crate::product_management::story::StoryId;
use chrono::{TimeZone, Utc};

fn story(num: u64, title: &str, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(num),
        title.to_string(),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
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

#[test]
fn default_scope_is_unchanged_full_board() {
    let stories = vec![
        story(1, "ready one", BoardState::Ready),
        story(2, "in-progress one", BoardState::InProgress(Stage::Dev)),
    ];
    let out = build_board_text(&[], &stories, &[], &BoardScope::default());
    assert!(out.contains("#1 ready one"));
    assert!(out.contains("#2 in-progress one"));
    assert!(out.contains(ColumnBucket::Ready.header()));
    assert!(out.contains(ColumnBucket::InProgress.header()));
}

#[test]
fn a_card_surfaces_its_acceptor_so_the_team_sees_who_accepts() {
    let mut s = story(77, "acceptor-visible", BoardState::Ready);
    s.acceptor = Some(crate::product_management::story::Acceptor::Owner);
    let out = build_board_text(&[], &[s], &[], &BoardScope::default());
    assert!(
        out.contains("#77 acceptor-visible · awaiting owner"),
        "the board card surfaces the acceptor (instead of the team reading None) so the acceptor is visible before desk-check; got:\n{out}"
    );
}

#[test]
fn a_card_without_an_acceptor_renders_no_acceptor_segment() {
    let out = build_board_text(
        &[],
        &[story(78, "no-acceptor-yet", BoardState::Ready)],
        &[],
        &BoardScope::default(),
    );
    assert!(
        out.contains("#78 no-acceptor-yet") && !out.contains("#78 no-acceptor-yet · awaiting"),
        "an unset acceptor adds no segment — only present acceptors render; got:\n{out}"
    );
}

#[test]
fn state_scope_renders_only_the_matching_bucket() {
    let stories = vec![
        story(1, "ready one", BoardState::Ready),
        story(2, "in-progress one", BoardState::InProgress(Stage::Dev)),
    ];
    let scope = BoardScope {
        state: Some(ColumnBucket::Ready),
        ..BoardScope::default()
    };
    let out = build_board_text(&[], &stories, &[], &scope);
    assert!(out.contains("#1 ready one"));
    assert!(!out.contains("#2 in-progress one"));
    assert!(!out.contains(ColumnBucket::InProgress.header()));
}

#[test]
fn iteration_scope_excludes_other_iterations() {
    let stories = vec![
        story(10, "iter-one story", BoardState::Ready),
        story(20, "iter-two story", BoardState::Ready),
    ];
    let iterations = vec![iteration_with(1, &[10]), iteration_with(2, &[20])];
    let scope = BoardScope {
        iteration: Some(1),
        ..BoardScope::default()
    };
    let out = build_board_text(&[], &stories, &iterations, &scope);
    assert!(out.contains("#10 iter-one story"));
    assert!(!out.contains("#20 iter-two story"));
}

#[test]
fn iteration_scope_derives_membership_from_the_container_not_the_story() {
    let stories = vec![story(30, "container-membered", BoardState::Ready)];
    let iterations = vec![iteration_with(5, &[30])];
    let scope = BoardScope {
        iteration: Some(5),
        ..BoardScope::default()
    };
    let out = build_board_text(&[], &stories, &iterations, &scope);
    assert!(
        out.contains("#30 container-membered"),
        "membership is the iteration container's concern (Iteration.stories); a story with no story-side field is still in scope when the iteration lists it; got:\n{out}"
    );
}

#[test]
fn row_limit_caps_entries_but_count_reflects_full_total() {
    let stories: Vec<Story> = (0..10)
        .map(|i| story(100 + i, &format!("ready {i}"), BoardState::Ready))
        .collect();
    let scope = BoardScope {
        state: Some(ColumnBucket::Ready),
        row_limit: Some(3),
        ..BoardScope::default()
    };
    let out = build_board_text(&[], &stories, &[], &scope);
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
                    BoardState::Ready
                } else {
                    BoardState::Accepted
                };
                story(1000 + i, &title, state)
            })
            .collect();
    let full = build_board_text(&[], &stories, &[], &BoardScope::default());
    let scoped = build_board_text(
        &[],
        &stories,
        &[],
        &BoardScope {
            state: Some(ColumnBucket::Ready),
            row_limit: Some(20),
            ..BoardScope::default()
        },
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
    assert!(!scoped.contains(ColumnBucket::Accepted.header()));
}
