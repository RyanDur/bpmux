use crate::board::domain::{Story, StoryState};
use crate::session::lessons::LessonIndex;
use crate::session::message::Message;
use crate::session::search::SearchFilters;
use crate::session::storage::{FileStorage, MemoryStorage, SqliteStorage, Storage};
use chrono::{TimeZone, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

static BPMUX_STORAGE_BACKEND_ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_storage_backend_env() -> MutexGuard<'static, ()> {
    BPMUX_STORAGE_BACKEND_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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
    let dir = tempfile::tempdir().unwrap();
    let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new(dir.path().to_path_buf()));
    (storage, dir)
}

fn sqlite_storage() -> (Arc<dyn Storage>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new(dir.path().to_path_buf()).unwrap());
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

            #[test]
            fn sqlite_storage() {
                let (storage, _dir) = super::sqlite_storage();
                $check(storage.as_ref());
            }
        }
    };
}

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

contract!(
    read_recent_returns_newest_first_after_offset,
    |storage: &dyn Storage| {
        for i in 0..5 {
            storage
                .append_message(&fresh_message(&format!("m{i}")))
                .unwrap();
        }
        let recent = storage.read_recent(2, 0, &|_: &Message| true).unwrap();
        assert_eq!(
            recent.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(),
            vec!["m4", "m3"]
        );
        let paged = storage.read_recent(2, 2, &|_: &Message| true).unwrap();
        assert_eq!(
            paged.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(),
            vec!["m2", "m1"]
        );
    }
);

contract!(
    read_recent_applies_the_predicate,
    |storage: &dyn Storage| {
        storage.append_message(&fresh_message("keep-1")).unwrap();
        storage.append_message(&fresh_message("drop")).unwrap();
        storage.append_message(&fresh_message("keep-2")).unwrap();
        let recent = storage
            .read_recent(10, 0, &|m: &Message| m.text.starts_with("keep"))
            .unwrap();
        assert_eq!(
            recent.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(),
            vec!["keep-2", "keep-1"]
        );
    }
);

contract!(
    read_recent_is_empty_when_no_messages,
    |storage: &dyn Storage| {
        let recent = storage.read_recent(10, 0, &|_: &Message| true).unwrap();
        assert!(recent.is_empty());
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

contract!(
    dir_returns_path_passed_at_construction,
    |storage: &dyn Storage| {
        let d = storage.dir();
        assert!(!d.as_os_str().is_empty());
    }
);

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

#[test]
fn session_with_memory_backend_env_skips_disk_io() {
    let _env_guard = lock_storage_backend_env();
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) =
        tokio::sync::broadcast::channel::<Message>(crate::session::BROADCAST_CHANNEL_CAPACITY);

    let prior = std::env::var("BPMUX_STORAGE_BACKEND").ok();
    unsafe {
        std::env::set_var("BPMUX_STORAGE_BACKEND", "memory");
    }

    let session = std::sync::Arc::new(crate::session::Session::new(
        dir.path().to_path_buf(),
        "test-dev".to_string(),
        tx,
    ));

    session
        .storage
        .append_message(&fresh_message("memory-mode-test"))
        .unwrap();

    let log_path = crate::session::log::log_path(dir.path());
    assert!(
        !log_path.exists(),
        "BPMUX_STORAGE_BACKEND=memory must keep storage pure-in-memory; log file at {log_path:?} should not exist after append"
    );

    let messages = session.storage.read_messages().unwrap();
    assert_eq!(
        messages.len(),
        1,
        "MemoryStorage retains the appended message in memory"
    );
    assert_eq!(messages[0].text, "memory-mode-test");

    unsafe {
        match prior {
            Some(v) => std::env::set_var("BPMUX_STORAGE_BACKEND", v),
            None => std::env::remove_var("BPMUX_STORAGE_BACKEND"),
        }
    }
}

#[test]
fn session_without_backend_env_writes_disk_io() {
    let _env_guard = lock_storage_backend_env();
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) =
        tokio::sync::broadcast::channel::<Message>(crate::session::BROADCAST_CHANNEL_CAPACITY);

    let prior = std::env::var("BPMUX_STORAGE_BACKEND").ok();
    unsafe {
        std::env::remove_var("BPMUX_STORAGE_BACKEND");
    }

    let session = std::sync::Arc::new(crate::session::Session::new(
        dir.path().to_path_buf(),
        "test-dev".to_string(),
        tx,
    ));

    session
        .storage
        .append_message(&fresh_message("file-mode-test"))
        .unwrap();

    let log_path = crate::session::log::log_path(dir.path());
    assert!(
        log_path.exists(),
        "default storage backend must write to disk at {log_path:?}"
    );

    if let Some(v) = prior {
        unsafe {
            std::env::set_var("BPMUX_STORAGE_BACKEND", v);
        }
    }
}

#[test]
fn session_with_sqlite_backend_env_selects_the_sqlite_db_not_the_flat_log() {
    let _env_guard = lock_storage_backend_env();
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) =
        tokio::sync::broadcast::channel::<Message>(crate::session::BROADCAST_CHANNEL_CAPACITY);

    let prior = std::env::var("BPMUX_STORAGE_BACKEND").ok();
    unsafe {
        std::env::set_var("BPMUX_STORAGE_BACKEND", "sqlite");
    }

    let session = std::sync::Arc::new(crate::session::Session::new(
        dir.path().to_path_buf(),
        "test-dev".to_string(),
        tx,
    ));

    session
        .storage
        .append_message(&fresh_message("sqlite-mode-test"))
        .unwrap();

    let sqlite_path = dir.path().join("session.db");
    assert!(
        sqlite_path.exists(),
        "BPMUX_STORAGE_BACKEND=sqlite must select the SQLite backend via the existing env (db at {sqlite_path:?} should exist after append)"
    );
    let jsonl_path = crate::session::log::log_path(dir.path());
    assert!(
        !jsonl_path.exists(),
        "the sqlite backend must NOT also write the flat jsonl log at {jsonl_path:?}"
    );

    let messages = session.storage.read_messages().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "sqlite-mode-test");

    unsafe {
        match prior {
            Some(v) => std::env::set_var("BPMUX_STORAGE_BACKEND", v),
            None => std::env::remove_var("BPMUX_STORAGE_BACKEND"),
        }
    }
}

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

#[test]
fn sqlite_concurrent_writers_on_the_same_db_file_lose_no_appends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let per_writer: usize = 50;

    let writer_a: Arc<dyn Storage> = Arc::new(SqliteStorage::new(path.clone()).unwrap());
    let writer_b: Arc<dyn Storage> = Arc::new(SqliteStorage::new(path.clone()).unwrap());

    let handle_a = {
        let writer = Arc::clone(&writer_a);
        std::thread::spawn(move || {
            for i in 0..per_writer {
                writer
                    .append_message(&fresh_message(&format!("a-{i}")))
                    .unwrap();
            }
        })
    };
    let handle_b = {
        let writer = Arc::clone(&writer_b);
        std::thread::spawn(move || {
            for i in 0..per_writer {
                writer
                    .append_message(&fresh_message(&format!("b-{i}")))
                    .unwrap();
            }
        })
    };
    handle_a.join().unwrap();
    handle_b.join().unwrap();

    let reader = SqliteStorage::new(path.clone()).unwrap();
    let all = reader.read_messages().unwrap();
    assert_eq!(
        all.len(),
        per_writer * 2,
        "two SQLite connections to the same db file (the server and a CLI verb are separate processes) writing concurrently must lose no appends — busy_timeout serializes the contending writers instead of erroring with SQLITE_BUSY"
    );
}
