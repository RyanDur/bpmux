#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::view::*;
use crate::product_management::board::column::ColumnBucket;
use crate::product_management::story::Story;
use crate::product_management::story::StoryState;
use chrono::{TimeZone, Utc};

fn story(num: u64, title: &str, state: StoryState) -> Story {
    Story::new_minimal(
        num,
        title.to_string(),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn in_iteration(mut s: Story, iteration: u64) -> Story {
    s.raw_labels
        .push(format!("parent-iteration-#{iteration}.1"));
    s.iteration_membership = Some((iteration, 1));
    s
}

#[test]
fn default_scope_is_unchanged_full_board() {
    let stories = vec![
        story(1, "ready one", StoryState::Ready),
        story(2, "in-progress one", StoryState::InProgress),
    ];
    let out = build_board_text(&stories, &BoardScope::default());
    assert!(out.contains("#1 ready one"));
    assert!(out.contains("#2 in-progress one"));
    assert!(out.contains(ColumnBucket::Ready.header()));
    assert!(out.contains(ColumnBucket::InProgress.header()));
}

#[test]
fn a_card_surfaces_its_acceptor_so_the_team_sees_who_accepts() {
    let mut s = story(77, "acceptor-visible", StoryState::Ready);
    s.raw_labels.push("awaiting owner".to_string());
    s.acceptor = Some(crate::product_management::story::Acceptor::Owner);
    let out = build_board_text(&[s], &BoardScope::default());
    assert!(
        out.contains("#77 acceptor-visible · awaiting owner"),
        "the board card surfaces the acceptor (instead of the team reading None) so the acceptor is visible before desk-check; got:\n{out}"
    );
}

#[test]
fn a_card_without_an_acceptor_renders_no_acceptor_segment() {
    let out = build_board_text(
        &[story(78, "no-acceptor-yet", StoryState::Ready)],
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
        story(1, "ready one", StoryState::Ready),
        story(2, "in-progress one", StoryState::InProgress),
    ];
    let scope = BoardScope {
        state: Some(ColumnBucket::Ready),
        ..BoardScope::default()
    };
    let out = build_board_text(&stories, &scope);
    assert!(out.contains("#1 ready one"));
    assert!(!out.contains("#2 in-progress one"));
    assert!(!out.contains(ColumnBucket::InProgress.header()));
}

#[test]
fn iteration_scope_excludes_other_iterations() {
    let stories = vec![
        in_iteration(story(10, "iter-one story", StoryState::Ready), 1),
        in_iteration(story(20, "iter-two story", StoryState::Ready), 2),
    ];
    let scope = BoardScope {
        iteration: Some(1),
        ..BoardScope::default()
    };
    let out = build_board_text(&stories, &scope);
    assert!(out.contains("#10 iter-one story"));
    assert!(!out.contains("#20 iter-two story"));
}

#[test]
fn row_limit_caps_entries_but_count_reflects_full_total() {
    let stories: Vec<Story> = (0..10)
        .map(|i| story(100 + i, &format!("ready {i}"), StoryState::Ready))
        .collect();
    let scope = BoardScope {
        state: Some(ColumnBucket::Ready),
        row_limit: Some(3),
        ..BoardScope::default()
    };
    let out = build_board_text(&stories, &scope);
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
                    StoryState::Ready
                } else {
                    StoryState::Accepted
                };
                story(1000 + i, &title, state)
            })
            .collect();
    let full = build_board_text(&stories, &BoardScope::default());
    let scoped = build_board_text(
        &stories,
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
