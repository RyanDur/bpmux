use chrono::{TimeZone, Utc};
use memory::lessons::LessonIndex;
use message::Message;
use message_log::MessageLog;
use std::sync::Arc;
use storage::DataGateways;

fn fresh_message(text: &str) -> Message {
    let mut msg = Message {
        from: "dev".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        ..Message::default()
    };
    msg.ensure_id();
    msg
}

fn ms(rfc3339: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .unwrap()
        .timestamp_millis()
}

fn sqlite_storage() -> (DataGateways, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let storage = storage::open(dir.path());
    (storage, dir)
}

macro_rules! contract {
    ($name:ident, $check:expr) => {
        mod $name {
            use super::*;

            #[test]
            fn sqlite_storage() {
                let (storage, _dir) = super::sqlite_storage();
                $check(&storage);
            }
        }
    };
}

contract!(
    append_then_read_returns_message_in_order,
    |storage: &DataGateways| {
        storage
            .conversation
            .append_message(&fresh_message("first"))
            .unwrap();
        storage
            .conversation
            .append_message(&fresh_message("second"))
            .unwrap();
        let msgs = storage.conversation.read_messages().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].text, "first");
        assert_eq!(msgs[1].text, "second");
    }
);

contract!(
    empty_storage_reads_empty_messages,
    |storage: &DataGateways| {
        let msgs = storage.conversation.read_messages().unwrap();
        assert!(msgs.is_empty());
    }
);

contract!(append_preserves_the_message_id, |storage: &DataGateways| {
    let mut msg = fresh_message("identified");
    msg.id = "11111111-1111-4111-8111-111111111111".to_string();
    storage.conversation.append_message(&msg).unwrap();
    let msgs = storage.conversation.read_messages().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(
        msgs[0].id, "11111111-1111-4111-8111-111111111111",
        "a message's id round-trips through storage unchanged"
    );
});

#[test]
fn sqlite_rejects_a_message_with_no_id() {
    let (storage, _dir) = sqlite_storage();
    let msg = Message {
        from: "dev".to_string(),
        text: "no identity".to_string(),
        ts: chrono::Utc::now(),
        ..Message::default()
    };
    assert!(msg.id.is_empty());
    assert!(
        storage.conversation.append_message(&msg).is_err(),
        "the id column refuses a message that was never minted an identity"
    );
}

#[test]
fn sqlite_rejects_a_non_uuid_message_id() {
    let (storage, _dir) = sqlite_storage();
    let mut msg = fresh_message("malformed");
    msg.id = "not-a-uuid".to_string();
    assert!(
        storage.conversation.append_message(&msg).is_err(),
        "the id column enforces uuid shape — a non-uuid string is refused"
    );
}

#[test]
fn sqlite_rejects_a_duplicate_message_id() {
    let (storage, _dir) = sqlite_storage();
    let mut first = fresh_message("first");
    first.id = "22222222-2222-4222-8222-222222222222".to_string();
    let mut second = fresh_message("second");
    second.id = "22222222-2222-4222-8222-222222222222".to_string();
    storage.conversation.append_message(&first).unwrap();
    assert!(
        storage.conversation.append_message(&second).is_err(),
        "the UNIQUE id column refuses a second message claiming the same identity"
    );
}

contract!(
    read_recent_returns_newest_first_after_offset,
    |storage: &DataGateways| {
        for i in 0..5 {
            storage
                .conversation
                .append_message(&fresh_message(&format!("m{i}")))
                .unwrap();
        }
        let recent = storage
            .conversation
            .read_recent(2, 0, &|_: &Message| true)
            .unwrap();
        assert_eq!(
            recent.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(),
            vec!["m4", "m3"]
        );
        let paged = storage
            .conversation
            .read_recent(2, 2, &|_: &Message| true)
            .unwrap();
        assert_eq!(
            paged.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(),
            vec!["m2", "m1"]
        );
    }
);

contract!(
    read_recent_applies_the_predicate,
    |storage: &DataGateways| {
        storage
            .conversation
            .append_message(&fresh_message("keep-1"))
            .unwrap();
        storage
            .conversation
            .append_message(&fresh_message("drop"))
            .unwrap();
        storage
            .conversation
            .append_message(&fresh_message("keep-2"))
            .unwrap();
        let recent = storage
            .conversation
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
    |storage: &DataGateways| {
        let recent = storage
            .conversation
            .read_recent(10, 0, &|_: &Message| true)
            .unwrap();
        assert!(recent.is_empty());
    }
);

contract!(load_lessons_default_when_empty, |storage: &DataGateways| {
    let idx = storage.lessons.load_lessons();
    assert!(idx.lessons.is_empty());
    assert!(idx.experiences.is_empty());
});

contract!(add_lesson_appends_to_index, |storage: &DataGateways| {
    storage
        .lessons
        .add_lesson(
            "topic-1",
            "lesson-text-1",
            "dev",
            vec!["evidence-1".to_string()],
        )
        .unwrap();
    let idx = storage.lessons.load_lessons();
    assert_eq!(idx.lessons.len(), 1);
    assert_eq!(idx.lessons[0].topic, "topic-1");
});

contract!(
    load_peer_lessons_default_when_empty,
    |storage: &DataGateways| {
        let idx = storage.lessons.load_peer_lessons("designer");
        assert!(idx.lessons.is_empty());
    }
);

contract!(
    save_then_load_peer_lessons_roundtrips,
    |storage: &DataGateways| {
        let mut idx = LessonIndex::default();
        idx.lessons.push(memory::lessons::AgentLesson {
            topic: "peer-topic".to_string(),
            lesson: "peer-lesson".to_string(),
            author_role: "designer".to_string(),
            evidence: vec![],
            established: "2026-05-11T12:00:00Z".to_string(),
            last_reinforced: None,
            reinforcement_count: 0,
            reinforcement_history: Vec::new(),
        });
        storage.lessons.save_peer_lessons("designer", &idx).unwrap();
        let loaded = storage.lessons.load_peer_lessons("designer");
        assert_eq!(loaded.lessons.len(), 1);
        assert_eq!(loaded.lessons[0].topic, "peer-topic");
    }
);

contract!(list_peers_reflects_saved_peers, |storage: &DataGateways| {
    let idx = LessonIndex::default();
    storage.lessons.save_peer_lessons("designer", &idx).unwrap();
    storage.lessons.save_peer_lessons("pm", &idx).unwrap();
    let peers = storage.lessons.list_peers();
    assert!(peers.contains(&"designer".to_string()));
    assert!(peers.contains(&"pm".to_string()));
});

fn seed_outbox_event(number: u64, label: &str) -> source_sync::QueuedEvent {
    use product_management::story::StoryId;
    use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
    let event = StoryEvent::new(
        EventMeta {
            actor: "dev".to_string(),
            actor_id: None,
            ts: Utc.with_ymd_and_hms(2026, 5, 11, 12, 0, 0).unwrap(),
        },
        StoryId::new(number),
        StoryEventKind::TagAdded {
            label: label.to_string(),
        },
    );
    source_sync::QueuedEvent::new(event)
}

contract!(load_outbox_empty_when_unset, |storage: &DataGateways| {
    let queue = storage.outbox.load_outbox().unwrap();
    assert!(queue.is_empty());
});

contract!(
    save_then_load_outbox_roundtrips,
    |storage: &DataGateways| {
        let events = vec![
            seed_outbox_event(100, "ready"),
            seed_outbox_event(200, "in-progress"),
        ];
        storage.outbox.save_outbox(&events).unwrap();
        let loaded = storage.outbox.load_outbox().unwrap();
        assert_eq!(loaded.len(), 2);
    }
);

contract!(save_outbox_overwrites_previous, |storage: &DataGateways| {
    storage
        .outbox
        .save_outbox(&[seed_outbox_event(100, "ready")])
        .unwrap();
    storage
        .outbox
        .save_outbox(&[
            seed_outbox_event(200, "icebox"),
            seed_outbox_event(300, "delivered"),
        ])
        .unwrap();
    let loaded = storage.outbox.load_outbox().unwrap();
    assert_eq!(loaded.len(), 2);
});

contract!(load_users_empty_when_unset, |storage: &DataGateways| {
    assert!(storage.users.load_users().unwrap().is_empty());
});

contract!(
    upsert_then_load_users_roundtrips,
    |storage: &DataGateways| {
        storage
            .users
            .upsert_user("sess-dev", "dev:claude-code", "dev")
            .unwrap();
        storage
            .users
            .upsert_user("sess-pm", "pm:claude-code", "pm")
            .unwrap();
        let mut loaded = storage.users.load_users().unwrap();
        loaded.sort_by(|a, b| a.user_id.cmp(&b.user_id));
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].user_id, "sess-dev");
        assert_eq!(loaded[0].name, "dev:claude-code");
        assert_eq!(loaded[0].role, "dev");
        assert_eq!(
            loaded[0].kind, "agent",
            "a routing-registered identity defaults to kind=agent; owner/system are the only non-agent kinds, seeded separately"
        );
        assert_eq!(loaded[0].last_delivered_at, None);
    }
);

contract!(
    upsert_user_updates_name_and_role_for_same_id,
    |storage: &DataGateways| {
        storage.users.upsert_user("sess-x", "dev", "dev").unwrap();
        storage
            .users
            .upsert_user("sess-x", "dev-renamed", "designer")
            .unwrap();
        let loaded = storage.users.load_users().unwrap();
        assert_eq!(loaded.len(), 1, "upsert on the same id must not duplicate");
        assert_eq!(loaded[0].name, "dev-renamed");
        assert_eq!(loaded[0].role, "designer");
    }
);

contract!(
    set_user_last_delivered_persists_the_floor,
    |storage: &DataGateways| {
        storage.users.upsert_user("sess-c", "dev", "dev").unwrap();
        let at = ms("2026-06-23T10:00:00Z");
        storage.users.set_user_last_delivered("sess-c", at).unwrap();
        let loaded = storage.users.load_users().unwrap();
        assert_eq!(loaded[0].last_delivered_at, Some(at));
    }
);

contract!(
    delete_users_removes_only_named_ids,
    |storage: &DataGateways| {
        storage.users.upsert_user("sess-a", "a", "dev").unwrap();
        storage.users.upsert_user("sess-b", "b", "pm").unwrap();
        storage.users.delete_users(&["sess-a".to_string()]).unwrap();
        let loaded = storage.users.load_users().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].user_id, "sess-b");
    }
);

contract!(
    load_surfaced_topics_empty_when_unset,
    |storage: &DataGateways| {
        assert!(storage.topics.load_surfaced_topics().is_empty());
    }
);

contract!(
    save_then_load_surfaced_topics_roundtrips,
    |storage: &DataGateways| {
        let mut topics = std::collections::HashSet::new();
        topics.insert("railway-errors".to_string());
        topics.insert("vertical-slices".to_string());
        storage.topics.save_surfaced_topics(&topics).unwrap();
        assert_eq!(storage.topics.load_surfaced_topics(), topics);
    }
);

#[test]
fn sqlite_backend_writes_the_db_and_the_hook_readable_tail() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = tokio::sync::broadcast::channel::<Message>(hub::BROADCAST_CHANNEL_CAPACITY);
    let hub = hub::Hub::new(dir.path().to_path_buf(), "test-dev".to_string(), tx);

    hub.conversation
        .append_message(&fresh_message("sqlite-mode-test"))
        .unwrap();

    let sqlite_path = dir.path().join("data.db");
    assert!(
        sqlite_path.exists(),
        "the sqlite backend must write to data.db at {sqlite_path:?}"
    );
    let jsonl_path = message_log::log_path(dir.path());
    assert!(
        jsonl_path.exists(),
        "the sqlite backend writes through to the jsonl tail the plugin hook reads at {jsonl_path:?}"
    );

    let messages = hub.conversation.read_messages().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "sqlite-mode-test");
}

#[test]
fn sqlite_concurrent_writers_on_the_same_db_file_lose_no_appends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let per_writer: usize = 50;

    let writer_a: Arc<dyn MessageLog> =
        Arc::new(message_log::data_gateway::ConversationDataGateway::new(
            Arc::new(database_support::DatabaseGateway::open(&path).unwrap()),
            path.clone(),
        ));
    let writer_b: Arc<dyn MessageLog> =
        Arc::new(message_log::data_gateway::ConversationDataGateway::new(
            Arc::new(database_support::DatabaseGateway::open(&path).unwrap()),
            path.clone(),
        ));

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

    let reader = message_log::data_gateway::ConversationDataGateway::new(
        Arc::new(database_support::DatabaseGateway::open(&path).unwrap()),
        path.clone(),
    );
    let all = reader.read_messages().unwrap();
    assert_eq!(
        all.len(),
        per_writer * 2,
        "two SQLite connections to the same db file (the server and a CLI verb are separate processes) writing concurrently must lose no appends — busy_timeout serializes the contending writers instead of erroring with SQLITE_BUSY"
    );
}
