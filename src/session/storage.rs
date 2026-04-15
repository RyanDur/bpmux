//! Storage abstraction — Strategy pattern.
//!
//! All persistence goes through this trait. The implementation
//! can be swapped without changing calling code.

use std::path::{Path, PathBuf};

use super::lessons::LessonIndex;
use super::log::LogError;
use super::message::Message;
use super::search::{SearchFilters, SearchResult};

/// The persistence interface. Core code depends on this trait,
/// never on a concrete storage mechanism.
pub trait Storage: Send + Sync {
    // Messages
    fn append_message(&self, msg: &Message) -> Result<(), LogError>;
    fn read_messages(&self) -> Result<Vec<Message>, LogError>;
    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult>;

    // Lessons
    fn load_lessons(&self) -> LessonIndex;
    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error>;
    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error>;

    // Peer lessons
    fn load_peer_lessons(&self, peer: &str) -> LessonIndex;
    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error>;
    fn list_peers(&self) -> Vec<String>;

    // Session state (generic key-value)
    fn load_state(&self, key: &str) -> Option<String>;
    fn save_state(&self, key: &str, value: &str);

    // Session directory (for backward compatibility during migration)
    fn dir(&self) -> &Path;
}

/// File-based storage — the current implementation.
pub struct FileStorage {
    dir: PathBuf,
}

impl FileStorage {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl Storage for FileStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        super::log::append_message(&self.dir, msg)
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        super::log::read_messages(&self.dir)
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        match self.read_messages() {
            Ok(messages) => super::search::search(&messages, filters),
            Err(_) => Vec::new(),
        }
    }

    fn load_lessons(&self) -> LessonIndex {
        super::lessons::load_index(&self.dir)
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        super::lessons::save_index(&self.dir, index)
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        super::lessons::add_lesson(&self.dir, topic, lesson, author_role, evidence)
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        super::lessons::load_peer_index(&self.dir, peer)
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        super::lessons::save_peer_index(&self.dir, peer, index)
    }

    fn list_peers(&self) -> Vec<String> {
        super::lessons::list_peer_names(&self.dir)
    }

    fn load_state(&self, key: &str) -> Option<String> {
        let path = self.dir.join(format!("{key}.json"));
        std::fs::read_to_string(&path).ok()
    }

    fn save_state(&self, key: &str, value: &str) {
        let safe_key = super::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        let tmp = self.dir.join(format!("{safe_key}.json.tmp"));
        if let Err(e) = std::fs::write(&tmp, value) {
            super::diag::log_diag(&format!("save_state write failed for {safe_key}: {e}"));
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &path) {
            super::diag::log_diag(&format!("save_state rename failed for {safe_key}: {e}"));
        }
    }

    fn dir(&self) -> &Path {
        &self.dir
    }
}
