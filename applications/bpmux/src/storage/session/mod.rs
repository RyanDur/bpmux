use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::product_management::board::PersistError;
use crate::product_management::board::orchestrator::QueuedEvent;

use super::{Storage, UserRecord, default_kind};
use crate::comprehension::search::{self, SearchFilters, SearchResult};
use crate::log::LogError;
use crate::memory::lessons::LessonIndex;
use crate::message::Message;

pub struct FileStorage {
    dir: PathBuf,
    outbox_path: Option<PathBuf>,
}

impl FileStorage {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            outbox_path: None,
        }
    }

    pub fn with_outbox_path(mut self, path: PathBuf) -> Self {
        self.outbox_path = Some(path);
        self
    }

    fn resolve_outbox_path(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(p) = &self.outbox_path {
            return Ok(p.clone());
        }
        crate::product_management::board::orchestrator::default_outbox_path()
    }

    fn write_users(&self, records: &[UserRecord]) -> Result<(), PersistError> {
        let path = self.dir.join("users.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(records)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

impl Storage for FileStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        crate::log::append_message(&self.dir, msg)
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        crate::log::read_messages(&self.dir)
    }

    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        crate::log::read_recent(&self.dir, limit, offset, matches)
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        match self.read_messages() {
            Ok(messages) => search::search(&messages, filters),
            Err(_) => Vec::new(),
        }
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        let log_path = self.dir.join("log.jsonl");
        match std::fs::remove_file(&log_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
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
        let safe_key = crate::memory::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        std::fs::read_to_string(&path).ok()
    }

    fn save_state(&self, key: &str, value: &str) {
        let safe_key = crate::memory::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        let tmp = self.dir.join(format!("{safe_key}.json.tmp"));
        if let Err(e) = std::fs::write(&tmp, value) {
            crate::diag::log_diag(&format!("save_state write failed for {safe_key}: {e}"));
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &path) {
            crate::diag::log_diag(&format!("save_state rename failed for {safe_key}: {e}"));
        }
    }

    fn load_surfaced_topics(&self) -> HashSet<String> {
        let path = self.dir.join("surfaced-topics.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    fn save_surfaced_topics(&self, topics: &HashSet<String>) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join("surfaced-topics.json");
        let json = serde_json::to_string(topics).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    fn dir(&self) -> &Path {
        &self.dir
    }

    fn load_users(&self) -> Result<Vec<UserRecord>, PersistError> {
        let path = self.dir.join("users.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = std::fs::read(&path)?;
        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn upsert_user(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError> {
        let mut records = self.load_users()?;
        match records.iter_mut().find(|r| r.session_id == id) {
            Some(existing) => {
                existing.name = name.to_string();
                existing.role = role.to_string();
            }
            None => records.push(UserRecord {
                session_id: id.to_string(),
                name: name.to_string(),
                role: role.to_string(),
                kind: default_kind(),
                last_delivered_at: None,
            }),
        }
        self.write_users(&records)
    }

    fn set_user_last_delivered(&self, id: &str, at: i64) -> Result<(), PersistError> {
        let mut records = self.load_users()?;
        if let Some(existing) = records.iter_mut().find(|r| r.session_id == id) {
            existing.last_delivered_at = Some(at);
        }
        self.write_users(&records)
    }

    fn delete_users(&self, ids: &[String]) -> Result<(), PersistError> {
        let mut records = self.load_users()?;
        records.retain(|r| !ids.iter().any(|id| id == &r.session_id));
        self.write_users(&records)
    }

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        let path = self.resolve_outbox_path()?;
        crate::product_management::board::orchestrator::load_outbox_from_disk(&path)
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        let path = self.resolve_outbox_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(events)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    fn load_bookmark(&self, role: &str) -> crate::bookmark::Bookmark {
        crate::bookmark::load_bookmark(&self.dir, role)
    }

    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &crate::bookmark::Bookmark,
    ) -> Result<(), std::io::Error> {
        crate::bookmark::save_bookmark(&self.dir, role, bookmark)
    }

    fn load_reactions(&self) -> crate::reactions::ReactionsStore {
        crate::reactions::load(&self.dir)
    }

    fn save_reactions(
        &self,
        store: &crate::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error> {
        crate::reactions::save(&self.dir, store)
    }
}

mod sqlite;
mod sqlite_bookmarks;
mod sqlite_engine;
mod sqlite_messages;
mod sqlite_meta;
mod sqlite_outbox;
mod sqlite_reactions;
mod sqlite_surfaced_topics;
mod sqlite_users;
pub use sqlite::*;

mod migrate;
pub use migrate::*;
#[cfg(test)]
mod migrate_tests;
#[cfg(test)]
mod sqlite_engine_behavior;
#[cfg(test)]
mod sqlite_store_behavior;
