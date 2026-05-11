#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

//! Tests for #1289: Store-trait DI refactor.
//!
//! Contract tests proving `FileStorage` + `MemoryStorage` behave
//! identically for every method on the `Storage` trait. Sister-pattern
//! to PM/designer agreement (2026-05-11 12:32 UTC trait-shape review):
//! both implementations must satisfy the same semantic contract;
//! file-system idiosyncrasies stay inside `FileStorage` per
//! `feedback_external_system_idiosyncrasies_at_boundary`.
//!
//! The shared `contract` macro runs the same assertion suite against
//! both impls so future trait extensions automatically verify parity.

use crate::board::domain::{Story, StoryState};
use crate::session::lessons::LessonIndex;
use crate::session::message::Message;
use crate::session::search::SearchFilters;
use crate::session::storage::{FileStorage, MemoryStorage, Storage};
use chrono::{TimeZone, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

fn fresh_message(text: &str) -> Message {
    Message {
        from: "dev".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        ..Message::default()
    }
}

fn file_storage() -> (Arc<dyn Storage>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    // #1289: inject per-test board-store + outbox paths so the contract
    // tests don't read/write Ryan's ~/.bpmux/board-store.json or
    // ~/.bpmux/board-outbox.json.
    let board_path = dir.path().join("board-store.json");
    let outbox_path = dir.path().join("board-outbox.json");
    let storage: Arc<dyn Storage> = Arc::new(
        FileStorage::new(dir.path().to_path_buf())
            .with_board_store_path(board_path)
            .with_outbox_path(outbox_path),
    );
    (storage, dir)
}

fn memory_storage() -> (Arc<dyn Storage>, tempfile::TempDir) {
    // MemoryStorage takes a marker path returned by dir(); use a
    // tempdir so it's unique per test. No I/O is performed against it.
    let dir = tempfile::tempdir().unwrap();
    let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new(dir.path().to_path_buf()));
    (storage, dir)
}

fn seed_story(number: u64, title: &str) -> Story {
    Story::new_minimal(
        number,
        title.to_string(),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 11, 12, 0, 0).unwrap(),
        "dev".to_string(),
    )
}

/// Run a contract assertion against both `FileStorage` + `MemoryStorage`.
/// Both must satisfy the same observable behavior; the macro fails if
/// either diverges so divergence is caught at test-write time.
macro_rules! contract {
    ($name:ident, $check:expr) => {
        mod $name {
            use super::*;

            #[test]
            fn file_storage() {
                let (storage, _dir) = super::file_storage();
                $check(storage.as_ref());
            }

            #[test]
            fn memory_storage() {
                let (storage, _dir) = super::memory_storage();
                $check(storage.as_ref());
            }
        }
    };
}

// ── Messages ──

contract!(
    append_then_read_returns_message_in_order,
    |storage: &dyn Storage| {
        storage.append_message(&fresh_message("first")).unwrap();
        storage.append_message(&fresh_message("second")).unwrap();
        let msgs = storage.read_messages().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].text, "first");
        assert_eq!(msgs[1].text, "second");
    }
);

contract!(
    empty_storage_reads_empty_messages,
    |storage: &dyn Storage| {
        let msgs = storage.read_messages().unwrap();
        assert!(msgs.is_empty());
    }
);

contract!(search_returns_matching_messages, |storage: &dyn Storage| {
    storage
        .append_message(&fresh_message("hello world"))
        .unwrap();
    storage.append_message(&fresh_message("goodbye")).unwrap();
    let results = storage.search_messages(&SearchFilters {
        query: "hello".to_string(),
        sender: None,
        after: None,
        before: None,
        channel: None,
        message_class: None,
        insight_only: false,
        owner_name: "dev".to_string(),
    });
    assert_eq!(results.len(), 1, "search for 'hello' returns 1 match");
});

// ── Lessons ──

contract!(load_lessons_default_when_empty, |storage: &dyn Storage| {
    let idx = storage.load_lessons();
    assert!(idx.lessons.is_empty());
    assert!(idx.experiences.is_empty());
});

contract!(
    save_then_load_lessons_roundtrips,
    |storage: &dyn Storage| {
        let mut idx = LessonIndex::default();
        idx.lessons.push(crate::session::lessons::AgentLesson {
            topic: "test-topic".to_string(),
            lesson: "test-lesson-text".to_string(),
            author_role: "dev".to_string(),
            evidence: vec!["e1".to_string()],
            established: "2026-05-11T12:00:00Z".to_string(),
            last_reinforced: None,
            reinforcement_count: 0,
            reinforcement_history: Vec::new(),
        });
        storage.save_lessons(&idx).unwrap();
        let loaded = storage.load_lessons();
        assert_eq!(loaded.lessons.len(), 1);
        assert_eq!(loaded.lessons[0].topic, "test-topic");
        assert_eq!(loaded.lessons[0].lesson, "test-lesson-text");
    }
);

contract!(add_lesson_appends_to_index, |storage: &dyn Storage| {
    storage
        .add_lesson(
            "topic-1",
            "lesson-text-1",
            "dev",
            vec!["evidence-1".to_string()],
        )
        .unwrap();
    let idx = storage.load_lessons();
    assert_eq!(idx.lessons.len(), 1);
    assert_eq!(idx.lessons[0].topic, "topic-1");
});

// ── Peer lessons ──

contract!(
    load_peer_lessons_default_when_empty,
    |storage: &dyn Storage| {
        let idx = storage.load_peer_lessons("designer");
        assert!(idx.lessons.is_empty());
    }
);

contract!(
    save_then_load_peer_lessons_roundtrips,
    |storage: &dyn Storage| {
        let mut idx = LessonIndex::default();
        idx.lessons.push(crate::session::lessons::AgentLesson {
            topic: "peer-topic".to_string(),
            lesson: "peer-lesson".to_string(),
            author_role: "designer".to_string(),
            evidence: vec![],
            established: "2026-05-11T12:00:00Z".to_string(),
            last_reinforced: None,
            reinforcement_count: 0,
            reinforcement_history: Vec::new(),
        });
        storage.save_peer_lessons("designer", &idx).unwrap();
        let loaded = storage.load_peer_lessons("designer");
        assert_eq!(loaded.lessons.len(), 1);
        assert_eq!(loaded.lessons[0].topic, "peer-topic");
    }
);

contract!(list_peers_reflects_saved_peers, |storage: &dyn Storage| {
    let idx = LessonIndex::default();
    storage.save_peer_lessons("designer", &idx).unwrap();
    storage.save_peer_lessons("pm", &idx).unwrap();
    let peers = storage.list_peers();
    assert!(peers.contains(&"designer".to_string()));
    assert!(peers.contains(&"pm".to_string()));
});

// ── Generic key-value state ──

contract!(
    load_state_returns_none_when_unset,
    |storage: &dyn Storage| {
        assert!(storage.load_state("unknown-key").is_none());
    }
);

contract!(save_then_load_state_roundtrips, |storage: &dyn Storage| {
    storage.save_state("my-key", "my-value");
    assert_eq!(storage.load_state("my-key").as_deref(), Some("my-value"));
});

contract!(
    save_state_overwrites_previous_value,
    |storage: &dyn Storage| {
        storage.save_state("k", "first");
        storage.save_state("k", "second");
        assert_eq!(storage.load_state("k").as_deref(), Some("second"));
    }
);

// ── dir() contract ──

contract!(
    dir_returns_path_passed_at_construction,
    |storage: &dyn Storage| {
        let d = storage.dir();
        assert!(!d.as_os_str().is_empty());
    }
);

// ── Board snapshot (#1289 Phase-1 step 1) ──

contract!(
    load_board_snapshot_empty_when_unset,
    |storage: &dyn Storage| {
        let snap = storage.load_board_snapshot().unwrap();
        assert!(snap.is_empty());
    }
);

contract!(
    save_then_load_board_snapshot_roundtrips,
    |storage: &dyn Storage| {
        let mut snap = HashMap::new();
        snap.insert(100, seed_story(100, "story-100"));
        snap.insert(200, seed_story(200, "story-200"));
        storage.save_board_snapshot(&snap).unwrap();
        let loaded = storage.load_board_snapshot().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get(&100).unwrap().title, "story-100");
        assert_eq!(loaded.get(&200).unwrap().title, "story-200");
    }
);

contract!(
    save_board_snapshot_overwrites_previous,
    |storage: &dyn Storage| {
        let mut snap_v1 = HashMap::new();
        snap_v1.insert(100, seed_story(100, "v1"));
        storage.save_board_snapshot(&snap_v1).unwrap();

        let mut snap_v2 = HashMap::new();
        snap_v2.insert(100, seed_story(100, "v2"));
        snap_v2.insert(200, seed_story(200, "added"));
        storage.save_board_snapshot(&snap_v2).unwrap();

        let loaded = storage.load_board_snapshot().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get(&100).unwrap().title, "v2");
    }
);

// ── Outbox (#1289 Phase-1 step 3) ──

fn seed_outbox_event(number: u64, label: &str) -> crate::board::orchestrator::QueuedEvent {
    use crate::board::events::{BoardEvent, EventMeta};
    use crate::board::peer::PeerId;
    let event = BoardEvent::TagAdded {
        meta: EventMeta {
            actor: "dev".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 5, 11, 12, 0, 0).unwrap(),
            peer_id: PeerId::local(),
        },
        number,
        label: label.to_string(),
    };
    crate::board::orchestrator::QueuedEvent::new(event, None)
}

contract!(load_outbox_empty_when_unset, |storage: &dyn Storage| {
    let queue = storage.load_outbox().unwrap();
    assert!(queue.is_empty());
});

contract!(save_then_load_outbox_roundtrips, |storage: &dyn Storage| {
    let events = vec![
        seed_outbox_event(100, "ready"),
        seed_outbox_event(200, "in-progress"),
    ];
    storage.save_outbox(&events).unwrap();
    let loaded = storage.load_outbox().unwrap();
    assert_eq!(loaded.len(), 2);
});

contract!(save_outbox_overwrites_previous, |storage: &dyn Storage| {
    storage
        .save_outbox(&[seed_outbox_event(100, "ready")])
        .unwrap();
    storage
        .save_outbox(&[
            seed_outbox_event(200, "icebox"),
            seed_outbox_event(300, "delivered"),
        ])
        .unwrap();
    let loaded = storage.load_outbox().unwrap();
    assert_eq!(loaded.len(), 2);
});

// ── Bookmark (#1289 Phase-2 step 1) ──

contract!(load_bookmark_default_when_unset, |storage: &dyn Storage| {
    let bm = storage.load_bookmark("dev");
    assert!(bm.current_task.is_empty());
    assert!(bm.decisions.is_empty());
});

contract!(
    save_then_load_bookmark_roundtrips,
    |storage: &dyn Storage| {
        let bm = crate::session::bookmark::Bookmark {
            current_task: "refactor storage trait".to_string(),
            decisions: vec!["use DI pattern".to_string()],
            open_questions: vec!["global vs session scope?".to_string()],
            next_step: "phase 2".to_string(),
            notes: "test note".to_string(),
            last_updated: "2026-05-11T13:00:00Z".to_string(),
        };
        storage.save_bookmark("dev", &bm).unwrap();
        let loaded = storage.load_bookmark("dev");
        assert_eq!(loaded.current_task, "refactor storage trait");
        assert_eq!(loaded.decisions.len(), 1);
    }
);

contract!(per_role_bookmark_isolation, |storage: &dyn Storage| {
    let dev_bm = crate::session::bookmark::Bookmark {
        current_task: "dev task".to_string(),
        ..Default::default()
    };
    let pm_bm = crate::session::bookmark::Bookmark {
        current_task: "pm task".to_string(),
        ..Default::default()
    };
    storage.save_bookmark("dev", &dev_bm).unwrap();
    storage.save_bookmark("pm", &pm_bm).unwrap();
    assert_eq!(storage.load_bookmark("dev").current_task, "dev task");
    assert_eq!(storage.load_bookmark("pm").current_task, "pm task");
});

// ── Pins (#1289 Phase-2 step 2) ──

contract!(load_pins_empty_when_unset, |storage: &dyn Storage| {
    let store = storage.load_pins();
    assert!(store.channels.is_empty());
});

contract!(save_then_load_pins_roundtrips, |storage: &dyn Storage| {
    let mut store = crate::session::pins::PinsStore::default();
    crate::session::pins::add_pin(
        &mut store,
        "broadcast",
        crate::session::pins::Pin {
            message_key: "2026-05-11T13:00:00Z|dev".to_string(),
            from: "dev".to_string(),
            ts: "2026-05-11T13:00:00Z".to_string(),
            text: "pinned message body".to_string(),
            pinned_by: "dev".to_string(),
            pinned_at: "2026-05-11T13:00:00Z".to_string(),
        },
    );
    storage.save_pins(&store).unwrap();
    let loaded = storage.load_pins();
    assert!(loaded.channels.contains_key("broadcast"));
});

// ── Reactions (#1289 Phase-2 step 3) ──

contract!(load_reactions_empty_when_unset, |storage: &dyn Storage| {
    let store = storage.load_reactions();
    assert!(store.channels.is_empty());
});

contract!(
    save_then_load_reactions_roundtrips,
    |storage: &dyn Storage| {
        let store = crate::session::reactions::ReactionsStore::default();
        storage.save_reactions(&store).unwrap();
        let loaded = storage.load_reactions();
        assert!(loaded.channels.is_empty());
    }
);

// ── Iteration markers (#1289 Phase-2 step 4) ──

contract!(
    load_iteration_markers_empty_when_unset,
    |storage: &dyn Storage| {
        let store = storage.load_iteration_markers();
        assert!(store.markers.is_empty());
    }
);

contract!(
    save_then_load_iteration_markers_roundtrips,
    |storage: &dyn Storage| {
        let mut store = crate::session::iteration_markers::IterationMarkersStore::default();
        crate::session::iteration_markers::add(
            &mut store,
            "Aquila".to_string(),
            "2026-05-11T13:00:00Z".to_string(),
        );
        storage.save_iteration_markers(&store).unwrap();
        let loaded = storage.load_iteration_markers();
        assert_eq!(loaded.markers.len(), 1);
        assert_eq!(loaded.markers[0].theme_name, "Aquila");
    }
);

// ── BoardStore ↔ Storage trait integration (#1289 Phase-1 step 2) ──

#[test]
fn board_store_roundtrips_via_memory_storage() {
    use crate::board::store::BoardStore;
    let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new(PathBuf::from(
        "/tmp/board-roundtrip-marker",
    )));
    let source = BoardStore::new();
    source.seed_all(vec![seed_story(100, "alpha"), seed_story(200, "beta")]);
    source.persist_to_storage(storage.as_ref()).unwrap();

    let restored = BoardStore::load_from_storage(storage.as_ref()).unwrap();
    let stories = restored.all_stories();
    assert_eq!(stories.len(), 2);
    assert!(
        stories
            .iter()
            .any(|s| s.number == 100 && s.title == "alpha")
    );
    assert!(stories.iter().any(|s| s.number == 200 && s.title == "beta"));
}

#[test]
fn board_store_load_from_empty_storage_returns_empty() {
    use crate::board::store::BoardStore;
    let storage: Arc<dyn Storage> =
        Arc::new(MemoryStorage::new(PathBuf::from("/tmp/board-empty-marker")));
    let store = BoardStore::load_from_storage(storage.as_ref()).unwrap();
    assert!(store.all_stories().is_empty());
}

// ── MemoryStorage-specific: per-instance isolation ──

#[test]
fn memory_storage_instances_are_isolated() {
    let a = MemoryStorage::new(PathBuf::from("/tmp/a-marker"));
    let b = MemoryStorage::new(PathBuf::from("/tmp/b-marker"));
    a.append_message(&fresh_message("to-a")).unwrap();
    let msgs_a = a.read_messages().unwrap();
    let msgs_b = b.read_messages().unwrap();
    assert_eq!(msgs_a.len(), 1);
    assert!(
        msgs_b.is_empty(),
        "MemoryStorage instances must not share state (per-test fixture invariant)"
    );
}
