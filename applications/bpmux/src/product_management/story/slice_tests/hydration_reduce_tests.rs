use chrono::{TimeZone, Utc};

use super::{apply, created, meta, minimal, seed, seeded, story_at};
use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{BoardState, FreeComment, Gate, StoryId};

#[test]
fn gate_comment_appended_event_accumulates() {
    let store = seeded();
    created(&store, 1, "t", BoardState::Ready);
    apply(
        &store,
        StoryEvent::GateCommentAppended {
            meta: meta("dev", 2026, "local"),
            id: StoryId::new(1),
            gate: Gate::Pickup,
            payload: Some("scope".to_string()),
        },
    );
    apply(
        &store,
        StoryEvent::GateCommentAppended {
            meta: meta("dev", 2026, "local"),
            id: StoryId::new(1),
            gate: Gate::Delivered,
            payload: Some("observation".to_string()),
        },
    );
    let story = story_at(&store, 1).unwrap();
    let gates = story.gate_comments.as_ref().unwrap();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0].gate, Gate::Pickup);
    assert_eq!(gates[1].gate, Gate::Delivered);
}

#[test]
fn free_comment_appended_event_accumulates() {
    let store = seeded();
    created(&store, 1, "t", BoardState::Ready);
    apply(
        &store,
        StoryEvent::FreeCommentAppended {
            meta: meta("designer", 2026, "local"),
            id: StoryId::new(1),
            body: "design note".to_string(),
        },
    );
    let story = story_at(&store, 1).unwrap();
    let free = story.free_comments.as_ref().unwrap();
    assert_eq!(free.len(), 1);
    assert_eq!(free[0].body, "design note");
    assert_eq!(free[0].author, "designer");
}

#[test]
fn body_fetched_event_populates_lazy_body() {
    let store = seeded();
    seed(&store, vec![minimal(1, "t", BoardState::Ready)]);
    assert_eq!(story_at(&store, 1).unwrap().body, None);
    apply(
        &store,
        StoryEvent::BodyFetched {
            meta: meta("dev", 2026, "local"),
            id: StoryId::new(1),
            body: "fetched body text".to_string(),
            etag: None,
        },
    );
    assert_eq!(
        story_at(&store, 1).unwrap().body.as_deref(),
        Some("fetched body text")
    );
}

#[test]
fn free_comments_fetched_event_replaces_lazy_field() {
    let store = seeded();
    seed(&store, vec![minimal(1, "t", BoardState::Ready)]);
    let comments = vec![FreeComment {
        body: "c1".to_string(),
        author: "pm".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
    }];
    apply(
        &store,
        StoryEvent::FreeCommentsFetched {
            meta: meta("dev", 2026, "local"),
            id: StoryId::new(1),
            comments: comments.clone(),
        },
    );
    assert_eq!(story_at(&store, 1).unwrap().free_comments, Some(comments));
}
