use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::board::orchestrator::QueuedEvent;
use crate::board::store::PersistError;

use super::{AgentRecord, Storage};
use crate::session::lessons::LessonIndex;
use crate::session::log::LogError;
use crate::session::message::Message;
use crate::session::search::{self, SearchFilters, SearchResult};

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
        crate::board::orchestrator::default_outbox_path()
    }

    fn write_agents(&self, records: &[AgentRecord]) -> Result<(), PersistError> {
        let path = self.dir.join("agents.json");
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
        crate::session::log::append_message(&self.dir, msg)
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        crate::session::log::read_messages(&self.dir)
    }

    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        crate::session::log::read_recent(&self.dir, limit, offset, matches)
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
        let safe_key = crate::session::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        std::fs::read_to_string(&path).ok()
    }

    fn save_state(&self, key: &str, value: &str) {
        let safe_key = crate::session::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        let tmp = self.dir.join(format!("{safe_key}.json.tmp"));
        if let Err(e) = std::fs::write(&tmp, value) {
            crate::session::diag::log_diag(&format!("save_state write failed for {safe_key}: {e}"));
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &path) {
            crate::session::diag::log_diag(&format!(
                "save_state rename failed for {safe_key}: {e}"
            ));
        }
    }

    fn dir(&self) -> &Path {
        &self.dir
    }

    fn load_agents(&self) -> Result<Vec<AgentRecord>, PersistError> {
        let path = self.dir.join("agents.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = std::fs::read(&path)?;
        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn upsert_agent(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError> {
        let mut records = self.load_agents()?;
        match records.iter_mut().find(|r| r.id == id) {
            Some(existing) => {
                existing.name = name.to_string();
                existing.role = role.to_string();
            }
            None => records.push(AgentRecord {
                id: id.to_string(),
                name: name.to_string(),
                role: role.to_string(),
                cursor: None,
            }),
        }
        self.write_agents(&records)
    }

    fn set_agent_cursor(&self, id: &str, cursor: &str) -> Result<(), PersistError> {
        let mut records = self.load_agents()?;
        if let Some(existing) = records.iter_mut().find(|r| r.id == id) {
            existing.cursor = Some(cursor.to_string());
        }
        self.write_agents(&records)
    }

    fn delete_agents(&self, ids: &[String]) -> Result<(), PersistError> {
        let mut records = self.load_agents()?;
        records.retain(|r| !ids.iter().any(|id| id == &r.id));
        self.write_agents(&records)
    }

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        let path = self.resolve_outbox_path()?;
        crate::board::orchestrator::load_outbox_from_disk(&path)
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

    fn load_bookmark(&self, role: &str) -> crate::session::bookmark::Bookmark {
        crate::session::bookmark::load_bookmark(&self.dir, role)
    }

    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &crate::session::bookmark::Bookmark,
    ) -> Result<(), std::io::Error> {
        crate::session::bookmark::save_bookmark(&self.dir, role, bookmark)
    }

    fn load_reactions(&self) -> crate::session::reactions::ReactionsStore {
        crate::session::reactions::load(&self.dir)
    }

    fn save_reactions(
        &self,
        store: &crate::session::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error> {
        crate::session::reactions::save(&self.dir, store)
    }
}

mod sqlite;
mod sqlite_agents;
mod sqlite_documents;
mod sqlite_engine;
mod sqlite_messages;
pub use sqlite::*;

mod migrate;
pub use migrate::*;
#[cfg(test)]
mod migrate_tests;
