use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::board::domain::Story;
use crate::board::orchestrator::QueuedEvent;
use crate::board::store::PersistError;

use super::Storage;
use crate::session::lessons::LessonIndex;
use crate::session::log::LogError;
use crate::session::message::Message;
use crate::session::search::{self, SearchFilters, SearchResult};

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
        crate::session::log::append_message(&self.dir, msg)
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        crate::session::log::read_messages(&self.dir)
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

pub struct MemoryStorage {
    dir: PathBuf,
    messages: Mutex<Vec<Message>>,
    lessons: Mutex<LessonIndex>,
    peer_lessons: Mutex<HashMap<String, LessonIndex>>,
    state: Mutex<HashMap<String, String>>,
    board_snapshot: Mutex<HashMap<u64, Story>>,
    outbox: Mutex<VecDeque<QueuedEvent>>,
    bookmarks: Mutex<HashMap<String, crate::session::bookmark::Bookmark>>,
    reactions: Mutex<crate::session::reactions::ReactionsStore>,
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
            reactions: Mutex::new(crate::session::reactions::ReactionsStore::default()),
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
        let new_entry = crate::session::lessons::AgentLesson {
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

    fn load_bookmark(&self, role: &str) -> crate::session::bookmark::Bookmark {
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
        bookmark: &crate::session::bookmark::Bookmark,
    ) -> Result<(), std::io::Error> {
        self.bookmarks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role.to_string(), bookmark.clone());
        Ok(())
    }

    fn load_reactions(&self) -> crate::session::reactions::ReactionsStore {
        self.reactions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn save_reactions(
        &self,
        store: &crate::session::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error> {
        *self.reactions.lock().unwrap_or_else(|e| e.into_inner()) = store.clone();
        Ok(())
    }
}
