use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use super::sqlite_bookmarks as bookmarks;
use super::sqlite_engine as engine;
use super::sqlite_messages as messages;
use super::sqlite_meta as meta;
use super::sqlite_outbox as outbox;
use super::sqlite_reactions as reactions;
use super::sqlite_surfaced_topics as surfaced_topics;
use super::sqlite_users as users;
use super::{Storage, UserRecord};
use crate::bookmark::Bookmark;
use crate::comprehension::search::{self, SearchFilters, SearchResult};
use crate::log::LogError;
use crate::memory::lessons::LessonIndex;
use crate::message::Message;
use crate::product_management::board::PersistError;
use crate::product_management::board::orchestrator::QueuedEvent;
use crate::reactions::ReactionsStore;

pub struct SqliteStorage {
    dir: PathBuf,
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    pub fn new(dir: PathBuf) -> rusqlite::Result<Self> {
        let conn = engine::open(&dir)?;
        users::seed_system(&conn)?;
        Ok(Self {
            dir,
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn resolve_sender(&self, conn: &Connection, msg: &Message) -> Option<String> {
        let session = if crate::routing::short_name(&msg.from) == "bpmux" {
            users::SYSTEM_SESSION_ID
        } else {
            msg.sender_id.as_deref()?
        };
        users::id_for_session(conn, session).ok().flatten()
    }
}

impl Storage for SqliteStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        let payload = serde_json::to_string(msg)?;
        let created_at = msg.ts.timestamp();
        let message_type = msg
            .message_type
            .to_wire()
            .unwrap_or_else(|| "chat".to_string());
        let conn = self.lock();
        let sender = self.resolve_sender(&conn, msg);
        messages::append(&conn, &msg.id, created_at, sender, &message_type, &payload)
            .map_err(engine::io_err)?;
        Ok(())
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        let payloads = messages::all_payloads_asc(&self.lock()).map_err(engine::io_err)?;
        payloads
            .iter()
            .map(|p| serde_json::from_str(p).map_err(LogError::from))
            .collect()
    }

    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        messages::read_recent(&self.lock(), limit, offset, matches)
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        match self.read_messages() {
            Ok(messages) => search::search(&messages, filters),
            Err(_) => Vec::new(),
        }
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        messages::clear(&self.lock())
            .map(|_| ())
            .map_err(engine::io_err)
    }

    fn load_lessons(&self) -> LessonIndex {
        crate::memory::lessons::load_index(&self.dir)
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        crate::memory::lessons::save_index(&self.dir, index)
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        crate::memory::lessons::add_lesson(&self.dir, topic, lesson, author_role, evidence)
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        crate::memory::lessons::load_peer_index(&self.dir, peer)
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        crate::memory::lessons::save_peer_index(&self.dir, peer, index)
    }

    fn list_peers(&self) -> Vec<String> {
        crate::memory::lessons::list_peer_names(&self.dir)
    }

    fn load_state(&self, key: &str) -> Option<String> {
        meta::get(&self.lock(), key).ok().flatten()
    }

    fn save_state(&self, key: &str, value: &str) {
        let _ = meta::put(&self.lock(), key, value);
    }

    fn load_surfaced_topics(&self) -> HashSet<String> {
        surfaced_topics::load(&self.lock())
    }

    fn save_surfaced_topics(&self, topics: &HashSet<String>) -> Result<(), std::io::Error> {
        surfaced_topics::replace(&self.lock(), topics)
    }

    fn dir(&self) -> &Path {
        &self.dir
    }

    fn load_users(&self) -> Result<Vec<UserRecord>, PersistError> {
        users::load_all(&self.lock()).map_err(engine::persist_err)
    }

    fn upsert_user(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError> {
        users::upsert(&self.lock(), id, name, role)
            .map(|_| ())
            .map_err(engine::persist_err)
    }

    fn set_user_last_delivered(&self, id: &str, at: i64) -> Result<(), PersistError> {
        users::set_last_delivered(&self.lock(), id, at)
            .map(|_| ())
            .map_err(engine::persist_err)
    }

    fn delete_users(&self, ids: &[String]) -> Result<(), PersistError> {
        users::delete(&self.lock(), ids)
            .map(|_| ())
            .map_err(engine::persist_err)
    }

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        outbox::load(&self.lock())
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        outbox::replace(&self.lock(), events)
    }

    fn load_bookmark(&self, role: &str) -> Bookmark {
        bookmarks::get(&self.lock(), role)
    }

    fn save_bookmark(&self, role: &str, bookmark: &Bookmark) -> Result<(), std::io::Error> {
        bookmarks::put(&self.lock(), role, bookmark)
    }

    fn load_reactions(&self) -> ReactionsStore {
        reactions::load(&self.lock())
    }

    fn save_reactions(&self, store: &ReactionsStore) -> Result<(), std::io::Error> {
        reactions::replace(&self.lock(), store)
    }
}
