use std::collections::VecDeque;

use crate::product_management::board::PersistError;
use crate::product_management::board::orchestrator::QueuedEvent;

use crate::memory::lessons::LessonIndex;

use crate::log::LogError;
use crate::message::Message;

mod backend;
mod global;
mod instrumented;
mod session;

pub use backend::StorageBackend;
pub use global::*;
pub use session::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserRecord {
    pub session_id: String,
    pub name: String,
    pub role: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub last_delivered_at: Option<i64>,
}

fn default_kind() -> String {
    "agent".to_string()
}

pub trait Storage: crate::awareness::SurfacedTopicsRepo + Send + Sync {
    fn append_message(&self, msg: &Message) -> Result<(), LogError>;
    fn read_messages(&self) -> Result<Vec<Message>, LogError>;
    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError>;
    fn clear_messages(&self) -> Result<(), std::io::Error>;

    fn load_lessons(&self) -> LessonIndex;
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

    fn load_users(&self) -> Result<Vec<UserRecord>, PersistError>;
    fn upsert_user(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError>;
    fn set_user_last_delivered(&self, id: &str, at: i64) -> Result<(), PersistError>;
    fn delete_users(&self, ids: &[String]) -> Result<(), PersistError>;

    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError>;
    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError>;
}

pub trait GlobalStore: Send + Sync {
    fn load_identity(&self, role: &str) -> crate::memory::identity::IdentityIndex;
    fn save_identity(
        &self,
        role: &str,
        index: &crate::memory::identity::IdentityIndex,
    ) -> Result<(), std::io::Error>;

    fn load_briefing(&self, role: &str) -> String;
    fn save_briefing(&self, role: &str, content: &str) -> Result<(), std::io::Error>;

    fn append_token_row(
        &self,
        row: &crate::telemetry::tokens::TokenRow,
    ) -> Result<(), crate::log::LogError>;
    fn read_token_rows(
        &self,
    ) -> Result<Vec<crate::telemetry::tokens::TokenRow>, crate::log::LogError>;

    fn load_usage(&self) -> std::collections::HashMap<String, crate::telemetry::usage::Record>;
    fn save_usage(
        &self,
        map: &std::collections::HashMap<String, crate::telemetry::usage::Record>,
    ) -> Result<(), std::io::Error>;

    fn ensure_auth_token(&self) -> std::io::Result<String>;
    fn read_auth_token(&self) -> std::io::Result<String>;
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) fn sqlite_test_storage(dir: &std::path::Path) -> std::sync::Arc<dyn Storage> {
    std::sync::Arc::new(SqliteStorage::new(dir.to_path_buf()).expect("sqlite test storage"))
}
