use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::board::domain::Story;
use crate::board::orchestrator::QueuedEvent;
use crate::board::store::PersistError;

use crate::session::lessons::LessonIndex;
use crate::session::log::LogError;
use crate::session::message::Message;
use crate::session::search::{self, SearchFilters, SearchResult};
use crate::session::storage::Storage;
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
