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

pub trait Storage: Send + Sync {
    fn append_message(&self, msg: &Message) -> Result<(), LogError>;
    fn read_messages(&self) -> Result<Vec<Message>, LogError>;
    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult>;
    fn clear_messages(&self) -> Result<(), std::io::Error>;

    fn load_lessons(&self) -> LessonIndex;
    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error>;
    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error>;

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex;
    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error>;
    fn list_peers(&self) -> Vec<String>;

    fn load_state(&self, key: &str) -> Option<String>;
    fn save_state(&self, key: &str, value: &str);

    fn dir(&self) -> &Path;

    fn load_board_snapshot(&self) -> Result<HashMap<u64, Story>, PersistError>;
    fn save_board_snapshot(&self, snapshot: &HashMap<u64, Story>) -> Result<(), PersistError>;

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError>;
    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError>;

    fn load_bookmark(&self, role: &str) -> super::bookmark::Bookmark;
    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &super::bookmark::Bookmark,
    ) -> Result<(), std::io::Error>;

    fn load_reactions(&self) -> super::reactions::ReactionsStore;
    fn save_reactions(
        &self,
        store: &super::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error>;
}

pub trait GlobalStore: Send + Sync {
    fn load_identity(&self, role: &str) -> super::identity::IdentityIndex;
    fn save_identity(
        &self,
        role: &str,
        index: &super::identity::IdentityIndex,
    ) -> Result<(), std::io::Error>;

    fn load_briefing(&self, role: &str) -> String;
    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error>;

    fn append_token_row(&self, row: &super::tokens::TokenRow) -> Result<(), super::log::LogError>;
    fn read_token_rows(&self) -> Result<Vec<super::tokens::TokenRow>, super::log::LogError>;

    fn load_usage(&self) -> std::collections::HashMap<String, super::usage::Record>;
    fn save_usage(
        &self,
        map: &std::collections::HashMap<String, super::usage::Record>,
    ) -> Result<(), std::io::Error>;

    fn ensure_auth_token(&self) -> std::io::Result<String>;
    fn read_auth_token(&self) -> std::io::Result<String>;
}

pub struct FileGlobalStore {
    identity_dir: Option<PathBuf>,
    briefings_dir: Option<PathBuf>,
    tokens_log_path: Option<PathBuf>,
    usage_path: Option<PathBuf>,
    auth_token_path: Option<PathBuf>,
}

impl Default for FileGlobalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FileGlobalStore {
    pub fn new() -> Self {
        Self {
            identity_dir: None,
            briefings_dir: None,
            tokens_log_path: None,
            usage_path: None,
            auth_token_path: None,
        }
    }

    pub fn with_identity_dir(mut self, dir: PathBuf) -> Self {
        self.identity_dir = Some(dir);
        self
    }

    pub fn with_briefings_dir(mut self, dir: PathBuf) -> Self {
        self.briefings_dir = Some(dir);
        self
    }

    pub fn with_tokens_log_path(mut self, path: PathBuf) -> Self {
        self.tokens_log_path = Some(path);
        self
    }

    pub fn with_usage_path(mut self, path: PathBuf) -> Self {
        self.usage_path = Some(path);
        self
    }

    pub fn with_auth_token_path(mut self, path: PathBuf) -> Self {
        self.auth_token_path = Some(path);
        self
    }

    fn resolve_identity_dir(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(d) = &self.identity_dir {
            return Ok(d.clone());
        }
        super::identity::default_identity_dir()
    }

    fn resolve_briefings_dir(&self) -> PathBuf {
        if let Some(d) = &self.briefings_dir {
            return d.clone();
        }
        super::briefing::briefings_dir()
    }

    fn resolve_tokens_log_path(&self) -> Result<PathBuf, super::log::LogError> {
        if let Some(p) = &self.tokens_log_path {
            return Ok(p.clone());
        }
        super::tokens::default_tokens_log_path()
    }

    fn resolve_usage_path(&self) -> Option<PathBuf> {
        if let Some(p) = &self.usage_path {
            return Some(p.clone());
        }
        super::usage::default_usage_path()
    }

    fn resolve_auth_token_path(&self) -> std::io::Result<PathBuf> {
        if let Some(p) = &self.auth_token_path {
            return Ok(p.clone());
        }
        super::auth_token::default_auth_token_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "home directory not found for auth_token",
            )
        })
    }
}

impl GlobalStore for FileGlobalStore {
    fn load_identity(&self, role: &str) -> super::identity::IdentityIndex {
        match self.resolve_identity_dir() {
            Ok(dir) => super::identity::load_identity(&dir, role),
            Err(_) => super::identity::IdentityIndex::default(),
        }
    }

    fn save_identity(
        &self,
        role: &str,
        index: &super::identity::IdentityIndex,
    ) -> Result<(), std::io::Error> {
        let dir = self.resolve_identity_dir()?;
        super::identity::save_identity(&dir, role, index)
    }

    fn load_briefing(&self, role: &str) -> String {
        super::briefing::load_briefing_in(&self.resolve_briefings_dir(), role)
    }

    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error> {
        super::briefing::save_briefing_in(&self.resolve_briefings_dir(), role, content)
    }

    fn append_token_row(&self, row: &super::tokens::TokenRow) -> Result<(), super::log::LogError> {
        let path = self.resolve_tokens_log_path()?;
        super::tokens::append_token_row(&path, row)
    }

    fn read_token_rows(&self) -> Result<Vec<super::tokens::TokenRow>, super::log::LogError> {
        let path = self.resolve_tokens_log_path()?;
        super::tokens::read_token_rows(&path)
    }

    fn load_usage(&self) -> std::collections::HashMap<String, super::usage::Record> {
        let Some(path) = self.resolve_usage_path() else {
            return std::collections::HashMap::new();
        };
        super::usage::load_from(&path)
    }

    fn save_usage(
        &self,
        map: &std::collections::HashMap<String, super::usage::Record>,
    ) -> Result<(), std::io::Error> {
        let Some(path) = self.resolve_usage_path() else {
            return Ok(());
        };
        super::usage::save_to(&path, map)
    }

    fn ensure_auth_token(&self) -> std::io::Result<String> {
        let path = self.resolve_auth_token_path()?;
        super::auth_token::ensure(&path)
    }

    fn read_auth_token(&self) -> std::io::Result<String> {
        let path = self.resolve_auth_token_path()?;
        super::auth_token::read(&path)
    }
}

pub struct MemoryGlobalStore {
    identity: Mutex<HashMap<String, super::identity::IdentityIndex>>,
    briefings: Mutex<HashMap<String, String>>,
    tokens: Mutex<Vec<super::tokens::TokenRow>>,
    usage: Mutex<HashMap<String, super::usage::Record>>,
}

impl Default for MemoryGlobalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryGlobalStore {
    pub fn new() -> Self {
        Self {
            identity: Mutex::new(HashMap::new()),
            briefings: Mutex::new(HashMap::new()),
            tokens: Mutex::new(Vec::new()),
            usage: Mutex::new(HashMap::new()),
        }
    }
}

impl GlobalStore for MemoryGlobalStore {
    fn load_identity(&self, role: &str) -> super::identity::IdentityIndex {
        self.identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    fn save_identity(
        &self,
        role: &str,
        index: &super::identity::IdentityIndex,
    ) -> Result<(), std::io::Error> {
        self.identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), index.clone());
        Ok(())
    }

    fn load_briefing(&self, role: &str) -> String {
        self.briefings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error> {
        self.briefings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), content.to_string());
        Ok(())
    }

    fn append_token_row(&self, row: &super::tokens::TokenRow) -> Result<(), super::log::LogError> {
        self.tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(row.clone());
        Ok(())
    }

    fn read_token_rows(&self) -> Result<Vec<super::tokens::TokenRow>, super::log::LogError> {
        Ok(self
            .tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone())
    }

    fn load_usage(&self) -> HashMap<String, super::usage::Record> {
        self.usage.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn save_usage(
        &self,
        map: &HashMap<String, super::usage::Record>,
    ) -> Result<(), std::io::Error> {
        *self.usage.lock().unwrap_or_else(|e| e.into_inner()) = map.clone();
        Ok(())
    }

    fn ensure_auth_token(&self) -> std::io::Result<String> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "MemoryGlobalStore does not support auth_token (process-owned secret \
             requires mode-0600 filesystem semantics); use FileGlobalStore with \
             BPMUX_AUTH_TOKEN_PATH for test isolation",
        ))
    }

    fn read_auth_token(&self) -> std::io::Result<String> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "MemoryGlobalStore does not support auth_token (process-owned secret \
             requires mode-0600 filesystem semantics); use FileGlobalStore with \
             BPMUX_AUTH_TOKEN_PATH for test isolation",
        ))
    }
}

pub struct FileStorage {
    dir: PathBuf,
    board_store_path: Option<PathBuf>,
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

    pub fn with_board_store_path(mut self, path: PathBuf) -> Self {
        self.board_store_path = Some(path);
        self
    }

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

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        let log_path = self.dir.join("log.jsonl");
        match std::fs::remove_file(&log_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
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

    fn load_bookmark(&self, role: &str) -> super::bookmark::Bookmark {
        super::bookmark::load_bookmark(&self.dir, role)
    }

    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &super::bookmark::Bookmark,
    ) -> Result<(), std::io::Error> {
        super::bookmark::save_bookmark(&self.dir, role, bookmark)
    }

    fn load_reactions(&self) -> super::reactions::ReactionsStore {
        super::reactions::load(&self.dir)
    }

    fn save_reactions(
        &self,
        store: &super::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error> {
        super::reactions::save(&self.dir, store)
    }
}

pub struct MemoryStorage {
    dir: PathBuf,
    messages: Mutex<Vec<Message>>,
    lessons: Mutex<LessonIndex>,
    peer_lessons: Mutex<HashMap<String, LessonIndex>>,
    state: Mutex<HashMap<String, String>>,
    board_snapshot: Mutex<HashMap<u64, Story>>,
    outbox: Mutex<VecDeque<QueuedEvent>>,
    bookmarks: Mutex<HashMap<String, super::bookmark::Bookmark>>,
    reactions: Mutex<super::reactions::ReactionsStore>,
}

impl MemoryStorage {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            messages: Mutex::new(Vec::new()),
            lessons: Mutex::new(LessonIndex::default()),
            peer_lessons: Mutex::new(HashMap::new()),
            state: Mutex::new(HashMap::new()),
            board_snapshot: Mutex::new(HashMap::new()),
            outbox: Mutex::new(VecDeque::new()),
            bookmarks: Mutex::new(HashMap::new()),
            reactions: Mutex::new(super::reactions::ReactionsStore::default()),
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

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        self.messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        Ok(())
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

    fn load_bookmark(&self, role: &str) -> super::bookmark::Bookmark {
        self.bookmarks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &super::bookmark::Bookmark,
    ) -> Result<(), std::io::Error> {
        self.bookmarks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), bookmark.clone());
        Ok(())
    }

    fn load_reactions(&self) -> super::reactions::ReactionsStore {
        self.reactions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn save_reactions(
        &self,
        store: &super::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error> {
        *self.reactions.lock().unwrap_or_else(|e| e.into_inner()) = store.clone();
        Ok(())
    }
}
