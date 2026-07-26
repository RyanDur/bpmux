use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use rusqlite::Connection;

use database_support as engine;
use database_support::DatabaseGateway;

use super::Storage;
use super::sqlite_meta as meta;
use awareness::db as surfaced_topics;
use memory::lessons::LessonIndex;
use message::Message;
use persist::PersistError;
use source_sync::QueuedEvent;
use source_sync::db as outbox;
use user_routing::db as users;
use user_routing::{UserRecord, UserRepo};

pub struct SqliteStorage {
    pub(super) dir: PathBuf,
    gateway: DatabaseGateway,
}

impl SqliteStorage {
    pub fn new(dir: PathBuf) -> rusqlite::Result<Self> {
        let gateway = DatabaseGateway::open(&dir)?;
        users::seed_system(&gateway.lock())?;
        Ok(Self { dir, gateway })
    }

    pub(super) fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.gateway.lock()
    }

    pub(super) fn resolve_sender(&self, conn: &Connection, msg: &Message) -> Option<String> {
        let user_id = if user_routing::short_name(&msg.from) == "bpmux" {
            users::SYSTEM_USER_ID
        } else {
            msg.sender_id.as_deref()?
        };
        users::id_for_user(conn, user_id).ok().flatten()
    }
}

impl memory::lessons::LessonRepo for SqliteStorage {
    fn load_lessons(&self) -> LessonIndex {
        memory::lessons::load_index(&self.dir)
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        memory::lessons::add_lesson(&self.dir, topic, lesson, author_role, evidence)
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        memory::lessons::load_peer_index(&self.dir, peer)
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        memory::lessons::save_peer_index(&self.dir, peer, index)
    }

    fn list_peers(&self) -> Vec<String> {
        memory::lessons::list_peer_names(&self.dir)
    }
}

impl Storage for SqliteStorage {
    fn load_state(&self, key: &str) -> Option<String> {
        meta::get(&self.lock(), key).ok().flatten()
    }

    fn save_state(&self, key: &str, value: &str) {
        let _ = meta::put(&self.lock(), key, value);
    }
}

impl UserRepo for SqliteStorage {
    fn load_users(&self) -> Result<Vec<UserRecord>, PersistError> {
        users::load_all(&self.lock()).map_err(engine::persist_err)
    }

    fn upsert_user(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError> {
        users::upsert(&self.lock(), id, name, role)
            .map(|_| ())
            .map_err(engine::persist_err)
    }

    fn set_user_last_delivered(&self, id: &str, at: i64) -> Result<(), PersistError> {
        users::set_last_delivered(&self.lock(), id, at)
            .map(|_| ())
            .map_err(engine::persist_err)
    }

    fn delete_users(&self, ids: &[String]) -> Result<(), PersistError> {
        users::delete(&self.lock(), ids)
            .map(|_| ())
            .map_err(engine::persist_err)
    }
}

impl source_sync::OutboxRepo for SqliteStorage {
    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        outbox::load(&self.lock())
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        outbox::replace(&self.lock(), events)
    }
}

impl awareness::SurfacedTopicsRepo for SqliteStorage {
    fn load_surfaced_topics(&self) -> HashSet<String> {
        surfaced_topics::load(&self.lock())
    }

    fn save_surfaced_topics(&self, topics: &HashSet<String>) -> Result<(), std::io::Error> {
        surfaced_topics::replace(&self.lock(), topics)
    }
}
