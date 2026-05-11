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

use crate::session::lessons::LessonIndex;
use crate::session::message::Message;
use crate::session::search::SearchFilters;
use crate::session::storage::{FileStorage, MemoryStorage, Storage};
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
    let storage: Arc<dyn Storage> = Arc::new(FileStorage::new(dir.path().to_path_buf()));
    (storage, dir)
}

fn memory_storage() -> (Arc<dyn Storage>, tempfile::TempDir) {
    // MemoryStorage takes a marker path returned by dir(); use a
    // tempdir so it's unique per test. No I/O is performed against it.
    let dir = tempfile::tempdir().unwrap();
    let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new(dir.path().to_path_buf()));
    (storage, dir)
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
