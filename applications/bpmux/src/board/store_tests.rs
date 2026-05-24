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
        etag: None,
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
fn tag_added_parent_iteration_label_rederives_iteration_membership_1553() {
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
        label: "parent-iteration-#9301.100".to_string(),
    });
    assert_eq!(
        store.story(1).unwrap().iteration_membership,
        Some((9301, 100))
    );
}

#[test]
fn tag_removed_parent_iteration_label_clears_iteration_membership_1553() {
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
        label: "parent-iteration-#9301.100".to_string(),
    });
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "parent-iteration-#9301.100".to_string(),
    });
    assert_eq!(store.story(1).unwrap().iteration_membership, None);
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

#[test]
fn parent_epic_set_event_assigns_parent_epic_field() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 100,
        title: "child".to_string(),
        body: "".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        number: 100,
        epic_number: 50,
    });
    assert_eq!(store.story(100).unwrap().parent_epic, Some(50));
}

#[test]
fn parent_epic_unset_event_clears_parent_epic_field() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 101,
        title: "child".to_string(),
        body: "".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        number: 101,
        epic_number: 51,
    });
    store.apply(&BoardEvent::ParentEpicUnset {
        meta: meta("pm", 2026, "local"),
        number: 101,
    });
    assert_eq!(store.story(101).unwrap().parent_epic, None);
}

#[test]
fn parent_epic_events_no_op_on_missing_story() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        number: 999,
        epic_number: 888,
    });
    store.apply(&BoardEvent::ParentEpicUnset {
        meta: meta("pm", 2026, "local"),
        number: 999,
    });
    assert!(store.story(999).is_none());
}

mod disk_persistence {
    use super::*;
    use tempfile::TempDir;

    fn sample_story(number: u64) -> Story {
        Story::new_minimal(
            number,
            format!("story #{number}"),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
            "pm:test".into(),
        )
    }

    #[test]
    fn load_from_disk_returns_empty_when_path_missing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("missing.json");
        let store = BoardStore::load_from_disk(&path).unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn persist_then_load_round_trips_stories() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("snap.json");

        let store = BoardStore::new();
        store.seed_all(vec![sample_story(101), sample_story(202)]);
        assert!(store.is_dirty());
        store.persist_to_disk(&path).unwrap();
        assert!(!store.is_dirty());
        assert!(path.exists());

        let restored = BoardStore::load_from_disk(&path).unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored.story(101).unwrap().title, "story #101");
        assert_eq!(restored.story(202).unwrap().title, "story #202");
    }

    #[test]
    fn persist_creates_parent_directory_atomically() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("a/b/c/board.json");
        let store = BoardStore::new();
        store.seed_all(vec![sample_story(1)]);
        store.persist_to_disk(&nested).unwrap();
        assert!(nested.exists());
        assert!(!nested.with_extension("tmp").exists());
    }

    #[test]
    fn apply_marks_dirty_so_periodic_snapshot_fires() {
        let store = BoardStore::new();
        assert!(!store.is_dirty());
        store.apply(&BoardEvent::TagRemoved {
            meta: meta("test", 2026, "p"),
            number: 7,
            label: "ready".into(),
        });
        assert!(store.is_dirty());
    }

    #[test]
    fn load_tolerates_empty_file_returns_empty_store() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.json");
        std::fs::write(&path, b"").unwrap();
        let store = BoardStore::load_from_disk(&path).unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn restart_simulation_hydrates_without_gh_call() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("session.json");

        let online_store = BoardStore::new();
        online_store.seed_all(vec![sample_story(42), sample_story(99)]);
        online_store.persist_to_disk(&path).unwrap();

        let offline_store = BoardStore::load_from_disk(&path).unwrap();
        assert_eq!(offline_store.len(), 2);
        assert!(
            offline_store.story(42).is_some(),
            "read paths must serve from disk-hydrate without GH"
        );
    }

    #[test]
    fn corrupted_json_surfaces_serde_error_not_silent_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("corrupted.json");
        std::fs::write(&path, b"{not valid json at all").unwrap();
        let err = BoardStore::load_from_disk(&path)
            .err()
            .expect("corruption must return Err, not silent empty Ok");
        assert!(
            matches!(err, crate::board::PersistError::Serde(_)),
            "corruption returns Serde-class Err so callers can distinguish from missing/empty"
        );
    }

    #[test]
    fn truncated_json_surfaces_serde_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("truncated.json");
        let well_formed_then_truncated = br#"{"42":{"number":42,"title":"valid","state":"#;
        std::fs::write(&path, well_formed_then_truncated).unwrap();
        let err = BoardStore::load_from_disk(&path)
            .err()
            .expect("mid-write truncation must return Err");
        assert!(
            matches!(err, crate::board::PersistError::Serde(_)),
            "mid-write truncation returns Serde Err"
        );
    }

    #[test]
    fn missing_file_returns_ok_empty_distinguished_from_corruption() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("does-not-exist.json");
        let store = BoardStore::load_from_disk(&path).unwrap();
        assert!(
            store.is_empty(),
            "missing file is healthy startup, not corruption"
        );
    }

    #[test]
    fn empty_file_returns_ok_empty_distinguished_from_corruption() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.json");
        std::fs::write(&path, b"").unwrap();
        let store = BoardStore::load_from_disk(&path).unwrap();
        assert!(
            store.is_empty(),
            "empty file is healthy startup, not corruption"
        );
    }
}
