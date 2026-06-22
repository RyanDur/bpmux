use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use super::Storage;
use super::sqlite_documents as documents;
use super::sqlite_engine as engine;
use crate::board::domain::Story;
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
        engine::append_message(&self.lock(), &payload).map_err(engine::io_err)?;
        Ok(())
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        let payloads = engine::all_payloads_asc(&self.lock()).map_err(engine::io_err)?;
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
        let conn = self.lock();
        let mut stmt = conn
            .prepare(engine::RECENT_DESC_SQL)
            .map_err(engine::io_err)?;
        let mut rows = stmt.query([]).map_err(engine::io_err)?;
        let mut out = Vec::new();
        let mut to_skip = offset;
        while let Some(row) = rows.next().map_err(engine::io_err)? {
            let payload: String = row.get(0).map_err(engine::io_err)?;
            let msg: Message = serde_json::from_str(&payload)?;
            if matches(&msg) {
                if to_skip > 0 {
                    to_skip -= 1;
                    continue;
                }
                if out.len() >= limit {
                    break;
                }
                out.push(msg);
            }
        }
        Ok(out)
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        match self.read_messages() {
            Ok(messages) => search::search(&messages, filters),
            Err(_) => Vec::new(),
        }
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        engine::clear_messages(&self.lock()).map_err(engine::io_err)
    }

    fn load_lessons(&self) -> LessonIndex {
        documents::load_doc(&self.lock(), documents::LESSONS_KEY).unwrap_or_default()
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        documents::save_doc(&self.lock(), documents::LESSONS_KEY, index)
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        let conn = self.lock();
        let mut index: LessonIndex =
            documents::load_doc(&conn, documents::LESSONS_KEY).unwrap_or_default();
        index.lessons.push(crate::session::lessons::AgentLesson {
            topic: topic.to_string(),
            lesson: lesson.to_string(),
            author_role: author_role.to_string(),
            evidence,
            established: chrono::Utc::now().to_rfc3339(),
            last_reinforced: None,
            reinforcement_count: 0,
            reinforcement_history: Vec::new(),
        });
        documents::save_doc(&conn, documents::LESSONS_KEY, &index)
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        documents::load_doc(&self.lock(), &documents::peer_key(peer)).unwrap_or_default()
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        documents::save_doc(&self.lock(), &documents::peer_key(peer), index)
    }

    fn list_peers(&self) -> Vec<String> {
        documents::list_peer_names(&self.lock())
    }

    fn load_state(&self, key: &str) -> Option<String> {
        engine::doc_get(&self.lock(), &documents::state_key(key))
            .ok()
            .flatten()
    }

    fn save_state(&self, key: &str, value: &str) {
        let _ = engine::doc_put(&self.lock(), &documents::state_key(key), value);
    }

    fn dir(&self) -> &Path {
        &self.dir
    }

    fn load_board_snapshot(&self) -> Result<HashMap<u64, Story>, PersistError> {
        Ok(documents::load_doc(&self.lock(), documents::BOARD_SNAPSHOT_KEY).unwrap_or_default())
    }

    fn save_board_snapshot(&self, snapshot: &HashMap<u64, Story>) -> Result<(), PersistError> {
        let value = serde_json::to_string(snapshot)?;
        engine::doc_put(&self.lock(), documents::BOARD_SNAPSHOT_KEY, &value)
            .map_err(engine::persist_err)
    }

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        Ok(documents::load_doc(&self.lock(), documents::OUTBOX_KEY).unwrap_or_default())
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        let value = serde_json::to_string(events)?;
        engine::doc_put(&self.lock(), documents::OUTBOX_KEY, &value).map_err(engine::persist_err)
    }

    fn load_bookmark(&self, role: &str) -> Bookmark {
        documents::load_doc(&self.lock(), &documents::bookmark_key(role)).unwrap_or_default()
    }

    fn save_bookmark(&self, role: &str, bookmark: &Bookmark) -> Result<(), std::io::Error> {
        documents::save_doc(&self.lock(), &documents::bookmark_key(role), bookmark)
    }

    fn load_reactions(&self) -> ReactionsStore {
        documents::load_doc(&self.lock(), documents::REACTIONS_KEY).unwrap_or_default()
    }

    fn save_reactions(&self, store: &ReactionsStore) -> Result<(), std::io::Error> {
        documents::save_doc(&self.lock(), documents::REACTIONS_KEY, store)
    }
}
