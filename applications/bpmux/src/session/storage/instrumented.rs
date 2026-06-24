use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::Storage;
use crate::session::lessons::LessonIndex;
use crate::session::log::LogError;
use crate::session::message::Message;
use crate::session::search::{SearchFilters, SearchResult};

pub struct InstrumentedStorage<S: Storage> {
    inner: S,
    log_path: Option<PathBuf>,
}

impl<S: Storage> InstrumentedStorage<S> {
    pub fn new(inner: S) -> Self {
        let log_path = std::env::var("BPMUX_STORAGE_LOG").ok().map(PathBuf::from);
        let this = Self { inner, log_path };
        if let Ok(messages) = this.inner.read_messages() {
            let lessons = this.inner.load_lessons();
            this.log(&format!(
                "startup: {} messages, {} experiences, {} lessons",
                messages.len(),
                lessons.experiences.len(),
                lessons.lessons.len(),
            ));
        }
        this
    }

    fn log(&self, line: &str) {
        let Some(path) = &self.log_path else {
            return;
        };
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{line}");
        }
    }
}

impl<S: Storage> Storage for InstrumentedStorage<S> {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        let start = Instant::now();
        let result = self.inner.append_message(msg);
        self.log(&format!(
            "append_message {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        let start = Instant::now();
        let result = self.inner.read_messages();
        if let Ok(ref msgs) = result {
            self.log(&format!(
                "read_messages {} msgs {:.1}ms",
                msgs.len(),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        result
    }

    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        let start = Instant::now();
        let result = self.inner.read_recent(limit, offset, matches);
        if let Ok(ref msgs) = result {
            self.log(&format!(
                "read_recent {} msgs (limit {limit} offset {offset}) {:.1}ms",
                msgs.len(),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        result
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        let start = Instant::now();
        let results = self.inner.search_messages(filters);
        self.log(&format!(
            "search '{}' → {} results {:.1}ms",
            filters.query,
            results.len(),
            start.elapsed().as_secs_f64() * 1000.0
        ));
        results
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        let start = Instant::now();
        let result = self.inner.clear_messages();
        self.log(&format!(
            "clear_messages {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn load_lessons(&self) -> LessonIndex {
        let start = Instant::now();
        let index = self.inner.load_lessons();
        self.log(&format!(
            "load_lessons {} lessons {:.1}ms",
            index.lessons.len(),
            start.elapsed().as_secs_f64() * 1000.0
        ));
        index
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        let start = Instant::now();
        let result = self.inner.save_lessons(index);
        self.log(&format!(
            "save_lessons {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        let start = Instant::now();
        let result = self.inner.add_lesson(topic, lesson, author_role, evidence);
        self.log(&format!(
            "add_lesson '{}' {:.1}ms",
            topic,
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        let start = Instant::now();
        let index = self.inner.load_peer_lessons(peer);
        self.log(&format!(
            "load_peer_lessons '{}' {:.1}ms",
            peer,
            start.elapsed().as_secs_f64() * 1000.0
        ));
        index
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        self.inner.save_peer_lessons(peer, index)
    }

    fn list_peers(&self) -> Vec<String> {
        self.inner.list_peers()
    }

    fn load_state(&self, key: &str) -> Option<String> {
        self.inner.load_state(key)
    }

    fn save_state(&self, key: &str, value: &str) {
        self.inner.save_state(key, value)
    }

    fn load_surfaced_topics(&self) -> std::collections::HashSet<String> {
        self.inner.load_surfaced_topics()
    }

    fn save_surfaced_topics(
        &self,
        topics: &std::collections::HashSet<String>,
    ) -> Result<(), std::io::Error> {
        self.inner.save_surfaced_topics(topics)
    }

    fn dir(&self) -> &Path {
        self.inner.dir()
    }

    fn load_users(
        &self,
    ) -> Result<Vec<crate::session::storage::UserRecord>, crate::board::store::PersistError> {
        let start = Instant::now();
        let result = self.inner.load_users();
        if let Ok(ref records) = result {
            self.log(&format!(
                "load_users {} users {:.1}ms",
                records.len(),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        result
    }

    fn upsert_user(
        &self,
        id: &str,
        name: &str,
        role: &str,
    ) -> Result<(), crate::board::store::PersistError> {
        let start = Instant::now();
        let result = self.inner.upsert_user(id, name, role);
        self.log(&format!(
            "upsert_user '{}' {:.1}ms",
            id,
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn set_user_last_delivered(
        &self,
        id: &str,
        at: i64,
    ) -> Result<(), crate::board::store::PersistError> {
        let start = Instant::now();
        let result = self.inner.set_user_last_delivered(id, at);
        self.log(&format!(
            "set_user_last_delivered '{}' {:.1}ms",
            id,
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn delete_users(&self, ids: &[String]) -> Result<(), crate::board::store::PersistError> {
        let start = Instant::now();
        let result = self.inner.delete_users(ids);
        self.log(&format!(
            "delete_users {} ids {:.1}ms",
            ids.len(),
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn load_outbox(
        &self,
    ) -> Result<
        std::collections::VecDeque<crate::board::orchestrator::QueuedEvent>,
        crate::board::store::PersistError,
    > {
        let start = Instant::now();
        let result = self.inner.load_outbox();
        if let Ok(ref queue) = result {
            self.log(&format!(
                "load_outbox {} events {:.1}ms",
                queue.len(),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        result
    }

    fn save_outbox(
        &self,
        events: &[crate::board::orchestrator::QueuedEvent],
    ) -> Result<(), crate::board::store::PersistError> {
        let start = Instant::now();
        let result = self.inner.save_outbox(events);
        self.log(&format!(
            "save_outbox {} events {:.1}ms",
            events.len(),
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn load_bookmark(&self, role: &str) -> crate::session::bookmark::Bookmark {
        let start = Instant::now();
        let result = self.inner.load_bookmark(role);
        self.log(&format!(
            "load_bookmark '{}' {:.1}ms",
            role,
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn save_bookmark(
        &self,
        role: &str,
        bookmark: &crate::session::bookmark::Bookmark,
    ) -> Result<(), std::io::Error> {
        let start = Instant::now();
        let result = self.inner.save_bookmark(role, bookmark);
        self.log(&format!(
            "save_bookmark '{}' {:.1}ms",
            role,
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn load_reactions(&self) -> crate::session::reactions::ReactionsStore {
        let start = Instant::now();
        let result = self.inner.load_reactions();
        self.log(&format!(
            "load_reactions {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn save_reactions(
        &self,
        store: &crate::session::reactions::ReactionsStore,
    ) -> Result<(), std::io::Error> {
        let start = Instant::now();
        let result = self.inner.save_reactions(store);
        self.log(&format!(
            "save_reactions {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }
}
