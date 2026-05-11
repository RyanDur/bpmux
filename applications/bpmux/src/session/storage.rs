//! Storage abstraction — Strategy pattern.
//!
//! All persistence goes through this trait. The implementation
//! can be swapped without changing calling code.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::board::domain::Story;
use crate::board::orchestrator::QueuedEvent;
use crate::board::store::PersistError;

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

    // #1289 Phase-1 extension: board snapshot (board-store.json).
    // FileStorage uses the configured `board_store_path` (env-overridable
    // via BPMUX_BOARD_STORE_PATH at call-time; injectable per-instance
    // via `with_board_store_path` for test isolation). MemoryStorage uses
    // per-instance in-memory state.
    fn load_board_snapshot(&self) -> Result<HashMap<u64, Story>, PersistError>;
    fn save_board_snapshot(&self, snapshot: &HashMap<u64, Story>) -> Result<(), PersistError>;

    // #1289 Phase-1 step 3: outbox queue (board-outbox.json).
    // FileStorage uses the configured `outbox_path` (env-overridable
    // via BPMUX_BOARD_OUTBOX_PATH at call-time; injectable per-instance
    // via `with_outbox_path` for test isolation). MemoryStorage uses
    // per-instance in-memory state. Disk-shape supports backward-compat
    // legacy-Vec<BoardEvent> parse via OutboxEntryOnDisk; new writes use
    // the QueuedEvent format directly.
    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError>;
    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError>;
}

/// File-based storage — the current implementation.
pub struct FileStorage {
    dir: PathBuf,
    /// #1289: per-instance board-store path override. `None` →
    /// `crate::board::store::default_store_path()` (env-or-home).
    /// `Some(p)` → use `p` directly. Tests inject per-test paths to
    /// avoid sharing `~/.bpmux/board-store.json`; production uses
    /// the default-path resolution.
    board_store_path: Option<PathBuf>,
    /// #1289: per-instance outbox-path override. Same shape as
    /// `board_store_path`; `None` → `default_outbox_path()` at
    /// call-time (BPMUX_BOARD_OUTBOX_PATH env-or-home).
    outbox_path: Option<PathBuf>,
}

impl FileStorage {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            board_store_path: None,
            outbox_path: None,
        }
    }

    /// #1289: opt-in path-injection for the board-snapshot store.
    /// Tests use this to point at a unique tempdir; production
    /// leaves it `None` so `default_store_path()` resolves at
    /// call-time (preserves env-override + home-default semantic).
    pub fn with_board_store_path(mut self, path: PathBuf) -> Self {
        self.board_store_path = Some(path);
        self
    }

    /// #1289 Phase-1 step 3: opt-in path-injection for the outbox.
    /// Sister of `with_board_store_path` at outbox-axis.
    pub fn with_outbox_path(mut self, path: PathBuf) -> Self {
        self.outbox_path = Some(path);
        self
    }

    fn resolve_board_store_path(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(p) = &self.board_store_path {
            return Ok(p.clone());
        }
        crate::board::store::default_store_path()
    }

    fn resolve_outbox_path(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(p) = &self.outbox_path {
            return Ok(p.clone());
        }
        crate::board::orchestrator::default_outbox_path()
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

    fn load_board_snapshot(&self) -> Result<HashMap<u64, Story>, PersistError> {
        let path = self.resolve_board_store_path()?;
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let bytes = std::fs::read(&path)?;
        if bytes.is_empty() {
            return Ok(HashMap::new());
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_board_snapshot(&self, snapshot: &HashMap<u64, Story>) -> Result<(), PersistError> {
        let path = self.resolve_board_store_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(snapshot)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
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
    board_snapshot: Mutex<HashMap<u64, Story>>,
    outbox: Mutex<VecDeque<QueuedEvent>>,
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
            board_snapshot: Mutex::new(HashMap::new()),
            outbox: Mutex::new(VecDeque::new()),
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

    fn load_board_snapshot(&self) -> Result<HashMap<u64, Story>, PersistError> {
        Ok(self
            .board_snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn save_board_snapshot(&self, snapshot: &HashMap<u64, Story>) -> Result<(), PersistError> {
        *self
            .board_snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = snapshot.clone();
        Ok(())
    }

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        Ok(self
            .outbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        let mut guard = self.outbox.lock().unwrap_or_else(|e| e.into_inner());
        *guard = events.iter().cloned().collect();
        Ok(())
    }
}
