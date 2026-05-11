//! Storage abstraction — Strategy pattern.
//!
//! All persistence goes through this trait. The implementation
//! can be swapped without changing calling code.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::lessons::LessonIndex;
use super::log::LogError;
use super::message::Message;
use super::search::{self, SearchFilters, SearchResult};

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
        let safe_key = super::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
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

/// In-memory storage — #1289 substrate-foundation for the Store-trait
/// DI refactor. Provides parity with `FileStorage` semantics on every
/// existing trait method without disk I/O.
///
/// Tests inject this via the launch helper so per-test backend startup
/// skips filesystem I/O (the macOS-WindowServer-class slowness root
/// cause from f9ebe47 / c8ba878 / 5bdb235 empirical evidence today).
///
/// `dir()` returns the path passed at construction — it's a marker
/// path tests use for any code that derives derived-state from
/// session-dir, but no I/O is performed against it.
pub struct MemoryStorage {
    dir: PathBuf,
    messages: Mutex<Vec<Message>>,
    lessons: Mutex<LessonIndex>,
    peer_lessons: Mutex<HashMap<String, LessonIndex>>,
    state: Mutex<HashMap<String, String>>,
}

impl MemoryStorage {
    /// `dir` is a marker path returned by `dir()`. No I/O is performed
    /// against it. Tests typically pass a `tempfile::tempdir()` path
    /// so the marker is unique per test.
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            messages: Mutex::new(Vec::new()),
            lessons: Mutex::new(LessonIndex::default()),
            peer_lessons: Mutex::new(HashMap::new()),
            state: Mutex::new(HashMap::new()),
        }
    }
}

impl Storage for MemoryStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        self.messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(msg.clone());
        Ok(())
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        Ok(self
            .messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        let messages = self
            .messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        search::search(&messages, filters)
    }

    fn load_lessons(&self) -> LessonIndex {
        self.lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        *self.lessons.lock().unwrap_or_else(|e| e.into_inner()) = index.clone();
        Ok(())
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        let new_entry = super::lessons::AgentLesson {
            topic: topic.to_string(),
            lesson: lesson.to_string(),
            author_role: author_role.to_string(),
            evidence,
            established: chrono::Utc::now().to_rfc3339(),
            last_reinforced: None,
            reinforcement_count: 0,
            reinforcement_history: Vec::new(),
        };
        self.lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .lessons
            .push(new_entry);
        Ok(())
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        self.peer_lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(peer)
            .cloned()
            .unwrap_or_default()
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        self.peer_lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(peer.to_string(), index.clone());
        Ok(())
    }

    fn list_peers(&self) -> Vec<String> {
        self.peer_lessons
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    fn load_state(&self, key: &str) -> Option<String> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    fn save_state(&self, key: &str, value: &str) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_string(), value.to_string());
    }

    fn dir(&self) -> &Path {
        &self.dir
    }
}
