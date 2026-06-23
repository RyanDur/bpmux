use std::collections::VecDeque;
use std::path::Path;

use crate::board::orchestrator::QueuedEvent;
use crate::board::store::PersistError;

use super::lessons::LessonIndex;
use super::log::LogError;
use super::message::Message;
use super::search::{SearchFilters, SearchResult};

mod global;
mod session;

pub use global::*;
pub use session::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub role: String,
    pub cursor: Option<String>,
}

pub trait Storage: Send + Sync {
    fn append_message(&self, msg: &Message) -> Result<(), LogError>;
    fn read_messages(&self) -> Result<Vec<Message>, LogError>;
    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError>;
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

    fn load_agents(&self) -> Result<Vec<AgentRecord>, PersistError>;
    fn upsert_agent(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError>;
    fn set_agent_cursor(&self, id: &str, cursor: &str) -> Result<(), PersistError>;
    fn delete_agents(&self, ids: &[String]) -> Result<(), PersistError>;

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
