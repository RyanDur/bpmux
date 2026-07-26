#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::DataGateways;
use message::{Message, MessageType};

fn store() -> (tempfile::TempDir, DataGateways) {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = crate::open(dir.path());
    (dir, storage)
}

fn chat(from: &str, sender_id: Option<&str>) -> Message {
    let mut msg = Message {
        from: from.to_string(),
        sender_id: sender_id.map(str::to_string),
        text: "hi".to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        ..Message::default()
    };
    msg.ensure_id();
    msg
}

#[test]
fn the_seeded_system_user_is_excluded_from_the_participant_roster() {
    let (_dir, storage) = store();
    assert!(
        storage.users.load_users().unwrap().is_empty(),
        "the system user is infrastructure, not a routable participant — load_users (which rebuilds routing identities) excludes it, so the backends stay at parity"
    );
}

#[test]
fn an_unresolvable_author_is_kept_never_dropped() {
    let (_dir, storage) = store();
    storage
        .conversation
        .append_message(&chat("rando", None))
        .unwrap();
    let stored = storage.conversation.read_messages().unwrap();
    assert_eq!(
        stored.len(),
        1,
        "an author the store cannot resolve is still persisted — the message is kept, never dropped to satisfy the FK"
    );
    assert_eq!(
        stored[0].from, "rando",
        "the kept message round-trips its original author verbatim"
    );
}

const FIX_QUEUED_VERDICT_ROW: &str = r#"{"id":0,"event":{"VerdictCast":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"verdict":{"Fail":{"step":"acceptance"}},"payload":"narrow viewport"}}}"#;

#[test]
fn an_outbox_row_round_trips_byte_identically_through_the_real_store() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = crate::open(dir.path());
    let raw = rusqlite::Connection::open(dir.path().join("data.db")).unwrap();
    raw.execute(
        "INSERT INTO outbox (event) VALUES (?1)",
        [FIX_QUEUED_VERDICT_ROW],
    )
    .unwrap();

    let loaded = storage.outbox.load_outbox().unwrap();
    assert_eq!(loaded.len(), 1);
    let entries: Vec<source_sync::QueuedEvent> = loaded.into_iter().collect();
    storage.outbox.save_outbox(&entries).unwrap();

    let stored: String = raw
        .query_row("SELECT event FROM outbox", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        stored, FIX_QUEUED_VERDICT_ROW,
        "the SQLite outbox row survives load + save byte-identically — SQLite has no legacy fallback, so this byte-freeze is its only protection"
    );
}

#[test]
fn reading_since_a_cutoff_serves_the_window_from_the_indexed_column() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = crate::open(dir.path());

    let mut old = chat("dev", None);
    old.text = "#1 started long ago".to_string();
    old.ts = chrono::Utc::now() - chrono::Duration::days(30);
    let mut recent = chat("dev", None);
    recent.text = "#2 started".to_string();
    recent.ts = chrono::Utc::now() - chrono::Duration::hours(2);
    let mut recent_activity = Message {
        from: "bpmux".to_string(),
        text: "#2 delivered".to_string(),
        ts: chrono::Utc::now() - chrono::Duration::hours(1),
        message_type: MessageType::Activity,
        ..Message::default()
    };
    recent_activity.ensure_id();

    storage.conversation.append_message(&old).unwrap();
    storage.conversation.append_message(&recent).unwrap();
    storage
        .conversation
        .append_message(&recent_activity)
        .unwrap();

    let cutoff = chrono::Utc::now() - chrono::Duration::days(14);
    let window = storage.conversation.read_messages_since(cutoff).unwrap();

    let texts: Vec<&str> = window.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, vec!["#2 started", "#2 delivered"]);
}

#[test]
fn search_hits_and_clusters_reconstruct_the_conversation_neighborhood() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = crate::open(dir.path());

    for text in ["opening remarks", "the needle sits here", "closing remarks"] {
        let mut m = chat("dev", None);
        m.text = text.to_string();
        storage.conversation.append_message(&m).unwrap();
    }

    let mut clusters: Vec<(Vec<String>, usize)> = Vec::new();
    storage
        .conversation
        .search_conversations("needle", None, 5, &mut |cluster, center| {
            let texts = cluster.iter().map(|m| m.text.clone()).collect();
            clusters.push((texts, center));
            true
        })
        .unwrap();

    let (texts, center) = clusters.pop().expect("one conversation carries the needle");
    assert!(clusters.is_empty(), "no other conversation matches");
    assert_eq!(
        texts,
        vec!["opening remarks", "the needle sits here", "closing remarks"],
        "the cluster carries the neighborhood in order"
    );
    assert_eq!(
        texts[center], "the needle sits here",
        "the match marks its own message"
    );
}

#[test]
fn addressing_lands_in_the_recipients_table_and_dies_with_its_message() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = crate::open(dir.path());

    let mut m = chat("dev", None);
    m.text = "for the pms and one individual".to_string();
    m.to = Some(vec!["pm".to_string(), "designer".to_string()]);
    m.to_ids = Some(vec!["6b3054e9-a6b4-4e5b-84db-f788ebdec25b".to_string()]);
    storage.conversation.append_message(&m).unwrap();

    let mut broadcast = chat("dev", None);
    broadcast.text = "to everyone".to_string();
    storage.conversation.append_message(&broadcast).unwrap();

    let recipients: Vec<(String, String)> = {
        let conn = storage.gateway.lock();
        let mut stmt = conn
            .prepare("SELECT kind, value FROM message_recipients ORDER BY kind, value")
            .unwrap();
        let rows = stmt
            .query_map([], |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())))
            .unwrap();
        rows.map(Result::unwrap).collect()
    };
    assert_eq!(
        recipients,
        vec![
            ("role".to_string(), "designer".to_string()),
            ("role".to_string(), "pm".to_string()),
            (
                "user".to_string(),
                "6b3054e9-a6b4-4e5b-84db-f788ebdec25b".to_string()
            ),
        ],
        "addressed roles and ids each get a row; the broadcast gets none"
    );

    storage.conversation.clear_messages().unwrap();
    let remaining: i64 = storage
        .gateway
        .lock()
        .query_row("SELECT COUNT(*) FROM message_recipients", [], |r| r.get(0))
        .unwrap();
    assert_eq!(remaining, 0, "recipients cascade away with their messages");
}

#[test]
fn addressee_search_narrows_candidates_through_the_recipients_table() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = crate::open(dir.path());

    let mut addressed = chat("dev", None);
    addressed.text = "the needle, for the pm".to_string();
    addressed.to = Some(vec!["pm".to_string()]);
    storage.conversation.append_message(&addressed).unwrap();

    let mut broadcast = chat("dev", None);
    broadcast.text = "the needle, for everyone".to_string();
    storage.conversation.append_message(&broadcast).unwrap();

    let mut centers = Vec::new();
    storage
        .conversation
        .search_conversations("needle", Some("pm"), 5, &mut |cluster, center| {
            centers.push(cluster[center].text.clone());
            true
        })
        .unwrap();

    assert_eq!(
        centers,
        vec!["the needle, for the pm"],
        "only the addressed message is a candidate; the broadcast never reaches the evaluator"
    );
}

#[test]
fn only_chat_messages_are_search_candidates() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = crate::open(dir.path());

    let mut noise = chat("bpmux", None);
    noise.text = "activity event carrying the needle".to_string();
    noise.message_type = message::MessageType::Activity;
    storage.conversation.append_message(&noise).unwrap();

    let mut spoken = chat("dev", None);
    spoken.text = "someone said the needle".to_string();
    storage.conversation.append_message(&spoken).unwrap();

    let mut centers = Vec::new();
    storage
        .conversation
        .search_conversations("needle", None, 5, &mut |cluster, center| {
            centers.push(cluster[center].text.clone());
            true
        })
        .unwrap();

    assert_eq!(
        centers,
        vec!["someone said the needle"],
        "the conversation is searchable; the plumbing around it is not"
    );
}

#[test]
fn story_transitions_dedupe_repeat_observations_and_read_by_window() {
    let (_dir, storage) = store();

    storage.transitions.record(7, "in-progress", 1_000);
    storage.transitions.record(7, "in-progress", 1_100);
    storage.transitions.record(7, "acceptance", 1_200);
    storage.transitions.record(8, "in-progress", 500);

    let rows = storage.transitions.since(900);
    assert_eq!(
        rows,
        vec![
            (7, "in-progress".to_string(), 1_000),
            (7, "acceptance".to_string(), 1_200),
        ],
        "repeat observation of the same state records once; the window filters the older story"
    );
}
