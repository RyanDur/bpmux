use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use super::sqlite_agents as agents;
use super::sqlite_bookmarks as bookmarks;
use super::sqlite_engine as engine;
use super::sqlite_messages as messages;
use super::sqlite_meta as meta;
use super::sqlite_outbox as outbox;
use super::sqlite_reactions as reactions;
use super::{AgentRecord, Storage};
use crate::board::orchestrator::QueuedEvent;
use crate::board::store::PersistError;
use crate::session::bookmark::Bookmark;
use crate::session::lessons::LessonIndex;
use crate::session::log::LogError;
use crate::session::message::Message;
use crate::session::reactions::ReactionsStore;
use crate::session::search::{self, SearchFilters, SearchResult};

pub struct SqliteStorage {
    dir: PathBuf,
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    pub fn new(dir: PathBuf) -> rusqlite::Result<Self> {
        let conn = engine::open(&dir)?;
        Ok(Self {
            dir,
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Storage for SqliteStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        let payload = serde_json::to_string(msg)?;
        let created_at = msg.ts.timestamp();
        let sender = msg
            .sender_id
            .as_deref()
            .unwrap_or_else(|| crate::transport::ws::short_name(&msg.from));
        let message_type = msg
            .message_type
            .to_wire()
            .unwrap_or_else(|| "chat".to_string());
        messages::append(&self.lock(), created_at, sender, &message_type, &payload)
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
        crate::session::lessons::load_index(&self.dir)
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        crate::session::lessons::save_index(&self.dir, index)
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        crate::session::lessons::add_lesson(&self.dir, topic, lesson, author_role, evidence)
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        crate::session::lessons::load_peer_index(&self.dir, peer)
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        crate::session::lessons::save_peer_index(&self.dir, peer, index)
    }

    fn list_peers(&self) -> Vec<String> {
        crate::session::lessons::list_peer_names(&self.dir)
    }

    fn load_state(&self, key: &str) -> Option<String> {
        meta::get(&self.lock(), key).ok().flatten()
    }

    fn save_state(&self, key: &str, value: &str) {
        let _ = meta::put(&self.lock(), key, value);
    }

    fn dir(&self) -> &Path {
        &self.dir
    }

    fn load_agents(&self) -> Result<Vec<AgentRecord>, PersistError> {
        agents::load_all(&self.lock()).map_err(engine::persist_err)
    }

    fn upsert_agent(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError> {
        agents::upsert(&self.lock(), id, name, role)
            .map(|_| ())
            .map_err(engine::persist_err)
    }

    fn set_agent_cursor(&self, id: &str, cursor: &str) -> Result<(), PersistError> {
        agents::set_cursor(&self.lock(), id, cursor)
            .map(|_| ())
            .map_err(engine::persist_err)
    }

    fn delete_agents(&self, ids: &[String]) -> Result<(), PersistError> {
        agents::delete(&self.lock(), ids)
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
