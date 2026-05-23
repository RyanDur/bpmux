#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(unsafe_code)]

use crate::board::domain::{Story, StoryState};
use crate::session::lessons::LessonIndex;
use crate::session::message::Message;
use crate::session::search::SearchFilters;
use crate::session::storage::{
    FileGlobalStore, FileStorage, GlobalStore, MemoryGlobalStore, MemoryStorage, Storage,
};
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

fn file_global() -> (Arc<dyn GlobalStore>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let identity_dir = dir.path().join("identity");
    let briefings_dir = dir.path().join("briefings");
    let tokens_log_path = dir.path().join("tokens").join("log.jsonl");
    let usage_path = dir.path().join("usage.json");
    let auth_token_path = dir.path().join("auth_token");
    let global: Arc<dyn GlobalStore> = Arc::new(
        FileGlobalStore::new()
            .with_identity_dir(identity_dir)
            .with_briefings_dir(briefings_dir)
            .with_tokens_log_path(tokens_log_path)
            .with_usage_path(usage_path)
            .with_auth_token_path(auth_token_path),
    );
    (global, dir)
}

fn memory_global() -> (Arc<dyn GlobalStore>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let global: Arc<dyn GlobalStore> = Arc::new(MemoryGlobalStore::new());
    (global, dir)
}

macro_rules! global_contract {
    ($name:ident, $check:expr) => {
        mod $name {
            use super::*;

            #[test]
            fn file_global() {
                let (global, _dir) = super::file_global();
                $check(global.as_ref());
            }

            #[test]
            fn memory_global() {
                let (global, _dir) = super::memory_global();
                $check(global.as_ref());
            }
        }
    };
}

global_contract!(
    load_identity_default_when_unset,
    |global: &dyn GlobalStore| {
        let idx = global.load_identity("dev");
        assert!(idx.values.is_empty());
    }
);

global_contract!(
    save_then_load_identity_roundtrips,
    |global: &dyn GlobalStore| {
        let mut idx = crate::session::identity::IdentityIndex::default();
        idx.values.push(crate::session::identity::IdentityValue {
            name: "test-name".to_string(),
            description: "test-value".to_string(),
            principles: vec![],
        });
        global.save_identity("dev", &idx).unwrap();
        let loaded = global.load_identity("dev");
        assert_eq!(loaded.values.len(), 1);
        assert_eq!(loaded.values[0].name, "test-name");
        assert_eq!(loaded.values[0].description, "test-value");
    }
);

global_contract!(per_role_identity_isolation, |global: &dyn GlobalStore| {
    let mut dev_idx = crate::session::identity::IdentityIndex::default();
    dev_idx
        .values
        .push(crate::session::identity::IdentityValue {
            name: "k".to_string(),
            description: "dev-val".to_string(),
            principles: vec![],
        });
    let mut pm_idx = crate::session::identity::IdentityIndex::default();
    pm_idx.values.push(crate::session::identity::IdentityValue {
        name: "k".to_string(),
        description: "pm-val".to_string(),
        principles: vec![],
    });
    global.save_identity("dev", &dev_idx).unwrap();
    global.save_identity("pm", &pm_idx).unwrap();
    assert_eq!(global.load_identity("dev").values[0].description, "dev-val");
    assert_eq!(global.load_identity("pm").values[0].description, "pm-val");
});

global_contract!(
    load_briefing_empty_when_unset,
    |global: &dyn GlobalStore| {
        let body = global.load_briefing("dev");
        assert!(body.is_empty());
    }
);

global_contract!(
    save_then_load_briefing_roundtrips,
    |global: &dyn GlobalStore| {
        global
            .save_briefing("dev", "# briefing\n\ntest content")
            .unwrap();
        let loaded = global.load_briefing("dev");
        assert_eq!(loaded, "# briefing\n\ntest content");
    }
);

global_contract!(per_role_briefing_isolation, |global: &dyn GlobalStore| {
    global.save_briefing("dev", "dev-content").unwrap();
    global.save_briefing("pm", "pm-content").unwrap();
    assert_eq!(global.load_briefing("dev"), "dev-content");
    assert_eq!(global.load_briefing("pm"), "pm-content");
});

fn sample_token_row(ts: &str, output: u64) -> crate::session::tokens::TokenRow {
    crate::session::tokens::TokenRow {
        ts: ts.to_string(),
        role: "dev".to_string(),
        harness: "claude-code".to_string(),
        model: "claude-opus-4-7".to_string(),
        input_tokens: 100,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 50,
        output_tokens: output,
        tool_call_count: 1,
        turn_duration_ms: None,
        transcript_line: None,
        session_id: None,
    }
}

global_contract!(
    read_token_rows_empty_when_unset,
    |global: &dyn GlobalStore| {
        let rows = global.read_token_rows().unwrap();
        assert!(rows.is_empty());
    }
);

global_contract!(
    append_then_read_token_rows_roundtrips,
    |global: &dyn GlobalStore| {
        let row = sample_token_row("2026-05-11T16:00:00Z", 200);
        global.append_token_row(&row).unwrap();
        let rows = global.read_token_rows().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], row);
    }
);

global_contract!(
    append_token_rows_preserves_order,
    |global: &dyn GlobalStore| {
        let r1 = sample_token_row("2026-05-11T16:00:00Z", 100);
        let r2 = sample_token_row("2026-05-11T16:01:00Z", 200);
        let r3 = sample_token_row("2026-05-11T16:02:00Z", 300);
        global.append_token_row(&r1).unwrap();
        global.append_token_row(&r2).unwrap();
        global.append_token_row(&r3).unwrap();
        let rows = global.read_token_rows().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].output_tokens, 100);
        assert_eq!(rows[1].output_tokens, 200);
        assert_eq!(rows[2].output_tokens, 300);
    }
);

global_contract!(load_usage_empty_when_unset, |global: &dyn GlobalStore| {
    let map = global.load_usage();
    assert!(map.is_empty());
});

global_contract!(
    save_then_load_usage_roundtrips,
    |global: &dyn GlobalStore| {
        let mut map = HashMap::new();
        map.insert(
            "mcp:start_story".to_string(),
            crate::session::usage::Record {
                count: 5,
                last_used: "2026-05-11T16:00:00Z".to_string(),
            },
        );
        global.save_usage(&map).unwrap();
        let loaded = global.load_usage();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get("mcp:start_story").unwrap().count, 5);
    }
);

global_contract!(
    save_usage_overwrites_previous,
    |global: &dyn GlobalStore| {
        let mut v1 = HashMap::new();
        v1.insert(
            "slash:tokens".to_string(),
            crate::session::usage::Record {
                count: 1,
                last_used: "2026-05-11T16:00:00Z".to_string(),
            },
        );
        global.save_usage(&v1).unwrap();

        let mut v2 = HashMap::new();
        v2.insert(
            "mcp:view_board".to_string(),
            crate::session::usage::Record {
                count: 10,
                last_used: "2026-05-11T16:01:00Z".to_string(),
            },
        );
        global.save_usage(&v2).unwrap();

        let loaded = global.load_usage();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.contains_key("mcp:view_board"));
        assert!(!loaded.contains_key("slash:tokens"));
    }
);

#[test]
fn file_global_ensure_auth_token_generates_and_reads_back() {
    let (global, _dir) = file_global();
    let token = global.ensure_auth_token().unwrap();
    assert!(!token.is_empty());
    assert_eq!(token.len(), 64, "32 bytes hex-encoded");
    let read_back = global.read_auth_token().unwrap();
    assert_eq!(token, read_back);
}

#[test]
fn file_global_ensure_auth_token_is_stable_across_calls() {
    let (global, _dir) = file_global();
    let t1 = global.ensure_auth_token().unwrap();
    let t2 = global.ensure_auth_token().unwrap();
    assert_eq!(t1, t2, "ensure returns existing token when present");
}

#[test]
fn memory_global_auth_token_returns_unsupported() {
    let global = MemoryGlobalStore::new();
    let err = global.ensure_auth_token().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    let err = global.read_auth_token().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
}

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
