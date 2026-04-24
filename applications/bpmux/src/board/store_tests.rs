#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use super::domain::{FreeComment, Gate, Story, StoryState, StoryType, Tag};
use super::events::{BoardEvent, EventMeta, event_wins};
use super::peer::PeerId;
use super::store::BoardStore;

fn meta(actor: &str, ts_year: i32, peer: &str) -> EventMeta {
    EventMeta {
        actor: actor.to_string(),
        ts: Utc.with_ymd_and_hms(ts_year, 1, 1, 0, 0, 0).unwrap(),
        peer_id: PeerId::named(peer),
    }
}

#[test]
fn store_starts_empty() {
    let store = BoardStore::new();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
    assert!(store.all_stories().is_empty());
}

#[test]
fn seed_all_populates_stories() {
    let store = BoardStore::new();
    let s1 = Story::new_minimal(
        1,
        "first".to_string(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "ryan".to_string(),
    );
    store.seed_all(vec![s1]);
    assert_eq!(store.len(), 1);
    assert_eq!(store.story(1).unwrap().title, "first");
}

#[test]
fn story_created_event_populates_store() {
    let store = BoardStore::new();
    let event = BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 500,
        title: "new story".to_string(),
        body: "body text".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    };
    store.apply(&event);
    let story = store.story(500).expect("story exists");
    assert_eq!(story.title, "new story");
    assert_eq!(story.state, StoryState::Ready);
    assert_eq!(story.body.as_deref(), Some("body text"));
    assert_eq!(story.filed_by, "pm");
}

#[test]
fn state_transitioned_event_updates_existing_story() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        number: 1,
        new_state: StoryState::InProgress,
    });
    assert_eq!(store.story(1).unwrap().state, StoryState::InProgress);
}

#[test]
fn state_transitioned_event_silently_drops_when_story_missing() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        number: 999,
        new_state: StoryState::InProgress,
    });
    assert!(store.story(999).is_none());
}

#[test]
fn gate_comment_appended_event_accumulates() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::GateCommentAppended {
        meta: meta("dev", 2026, "local"),
        number: 1,
        gate: Gate::Pickup,
        payload: Some("scope".to_string()),
    });
    store.apply(&BoardEvent::GateCommentAppended {
        meta: meta("dev", 2026, "local"),
        number: 1,
        gate: Gate::Delivered,
        payload: Some("observation".to_string()),
    });
    let story = store.story(1).unwrap();
    let gates = story.gate_comments.as_ref().unwrap();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0].gate, Gate::Pickup);
    assert_eq!(gates[1].gate, Gate::Delivered);
}

#[test]
fn free_comment_appended_event_accumulates() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::FreeCommentAppended {
        meta: meta("designer", 2026, "local"),
        number: 1,
        body: "design note".to_string(),
    });
    let story = store.story(1).unwrap();
    let free = story.free_comments.as_ref().unwrap();
    assert_eq!(free.len(), 1);
    assert_eq!(free[0].body, "design note");
    assert_eq!(free[0].author, "designer");
}

#[test]
fn body_fetched_event_populates_lazy_body() {
    let store = BoardStore::new();
    store.seed_all(vec![Story::new_minimal(
        1,
        "t".to_string(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )]);
    assert_eq!(store.story(1).unwrap().body, None);
    store.apply(&BoardEvent::BodyFetched {
        meta: meta("dev", 2026, "local"),
        number: 1,
        body: "fetched body text".to_string(),
    });
    assert_eq!(
        store.story(1).unwrap().body.as_deref(),
        Some("fetched body text")
    );
}

#[test]
fn free_comments_fetched_event_replaces_lazy_field() {
    let store = BoardStore::new();
    store.seed_all(vec![Story::new_minimal(
        1,
        "t".to_string(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )]);
    let comments = vec![FreeComment {
        body: "c1".to_string(),
        author: "pm".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
    }];
    store.apply(&BoardEvent::FreeCommentsFetched {
        meta: meta("dev", 2026, "local"),
        number: 1,
        comments: comments.clone(),
    });
    assert_eq!(store.story(1).unwrap().free_comments, Some(comments));
}

#[test]
fn stories_in_state_filters_correctly() {
    let store = BoardStore::new();
    store.seed_all(vec![
        Story::new_minimal(
            1,
            "a".to_string(),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
        Story::new_minimal(
            2,
            "b".to_string(),
            StoryState::Delivered,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
        Story::new_minimal(
            3,
            "c".to_string(),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
    ]);
    let ready = store.stories_in_state(StoryState::Ready);
    assert_eq!(ready.len(), 2);
    let mut nums: Vec<u64> = ready.iter().map(|s| s.number).collect();
    nums.sort();
    assert_eq!(nums, vec![1, 3]);
}

#[test]
fn event_wins_later_ts_beats_earlier() {
    let earlier = meta("a", 2025, "local");
    let later = meta("b", 2026, "local");
    assert!(event_wins(&later, &earlier));
    assert!(!event_wins(&earlier, &later));
}

#[test]
fn event_wins_tiebreak_by_peer_id_lexical_when_ts_equal() {
    let a = EventMeta {
        actor: "x".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        peer_id: PeerId::named("aaa"),
    };
    let b = EventMeta {
        actor: "y".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        peer_id: PeerId::named("zzz"),
    };
    assert!(event_wins(&b, &a));
    assert!(!event_wins(&a, &b));
}

#[test]
fn peer_id_local_is_the_default_for_mvp() {
    let p = PeerId::local();
    assert_eq!(p.as_str(), "local");
}

#[tokio::test]
async fn subscribe_delivers_applied_events_to_listeners() {
    let store = BoardStore::new();
    let mut rx = store.subscribe();
    let event = BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "sub".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    };
    store.apply(&event);
    let received = rx.recv().await.expect("receive");
    assert_eq!(received, event);
}

#[tokio::test]
async fn subscribe_fans_out_to_multiple_listeners() {
    let store = BoardStore::new();
    let mut a = store.subscribe();
    let mut b = store.subscribe();
    let event = BoardEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        number: 7,
        new_state: StoryState::Delivered,
    };
    store.seed_all(vec![Story::new_minimal(
        7,
        "t".to_string(),
        StoryState::Ready,
        chrono::Utc::now(),
        "pm".to_string(),
    )]);
    store.apply(&event);
    let a_event = a.recv().await.expect("a recv");
    let b_event = b.recv().await.expect("b recv");
    assert_eq!(a_event, event);
    assert_eq!(b_event, event);
}

#[tokio::test]
async fn receiver_count_tracks_live_subscribers() {
    let store = BoardStore::new();
    assert_eq!(store.receiver_count(), 0);
    let _rx = store.subscribe();
    assert_eq!(store.receiver_count(), 1);
    let _rx2 = store.subscribe();
    assert_eq!(store.receiver_count(), 2);
    drop(_rx);
    assert_eq!(store.receiver_count(), 1);
}

#[tokio::test]
async fn apply_succeeds_when_no_subscribers_are_attached() {
    let store = BoardStore::new();
    let event = BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 99,
        title: "no-subs".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    };
    store.apply(&event);
    assert!(store.story(99).is_some());
}

#[test]
fn tag_added_event_appends_to_story_tags() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    assert_eq!(store.story(1).unwrap().tags, vec![Tag::Blocked]);
}

#[test]
fn tag_added_unknown_label_is_dropped() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "high-priority".to_string(),
    });
    assert!(store.story(1).unwrap().tags.is_empty());
}

#[test]
fn tag_added_is_idempotent() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    assert_eq!(store.story(1).unwrap().tags, vec![Tag::Blocked]);
}

#[test]
fn tag_removed_event_strips_tag() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    assert!(store.story(1).unwrap().tags.is_empty());
}

#[test]
fn tag_removed_unknown_label_is_noop() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "high-priority".to_string(),
    });
    assert_eq!(store.story(1).unwrap().tags, vec![Tag::Blocked]);
}

#[test]
fn story_created_event_carries_story_type() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Bug,
    });
    assert_eq!(store.story(1).unwrap().story_type, StoryType::Bug);
}
