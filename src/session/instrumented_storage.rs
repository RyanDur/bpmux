//! Decorator over Storage that logs timing and data volume.

use std::path::Path;
use std::time::Instant;

use super::lessons::LessonIndex;
use super::log::LogError;
use super::message::Message;
use super::search::{SearchFilters, SearchResult};
use super::storage::Storage;

pub struct InstrumentedStorage<S: Storage> {
    inner: S,
}

impl<S: Storage> InstrumentedStorage<S> {
    pub fn new(inner: S) -> Self {
        // Log startup metrics
        if let Ok(messages) = inner.read_messages() {
            let lessons = inner.load_lessons();
            eprintln!(
                "Storage: {} messages, {} experiences, {} lessons",
                messages.len(),
                lessons.experiences.len(),
                lessons.lessons.len(),
            );
        }
        Self { inner }
    }
}

impl<S: Storage> Storage for InstrumentedStorage<S> {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        let start = Instant::now();
        let result = self.inner.append_message(msg);
        eprintln!(
            "Storage: append_message {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
        result
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        let start = Instant::now();
        let result = self.inner.read_messages();
        if let Ok(ref msgs) = result {
            eprintln!(
                "Storage: read_messages {} msgs {:.1}ms",
                msgs.len(),
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
        result
    }

    fn search_messages(&self, filters: &SearchFilters) -> Vec<SearchResult> {
        let start = Instant::now();
        let results = self.inner.search_messages(filters);
        eprintln!(
            "Storage: search '{}' → {} results {:.1}ms",
            filters.query,
            results.len(),
            start.elapsed().as_secs_f64() * 1000.0
        );
        results
    }

    fn load_lessons(&self) -> LessonIndex {
        let start = Instant::now();
        let index = self.inner.load_lessons();
        eprintln!(
            "Storage: load_lessons {} lessons {:.1}ms",
            index.lessons.len(),
            start.elapsed().as_secs_f64() * 1000.0
        );
        index
    }

    fn save_lessons(&self, index: &LessonIndex) -> Result<(), std::io::Error> {
        let start = Instant::now();
        let result = self.inner.save_lessons(index);
        eprintln!(
            "Storage: save_lessons {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
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
        eprintln!(
            "Storage: add_lesson '{}' {:.1}ms",
            topic,
            start.elapsed().as_secs_f64() * 1000.0
        );
        result
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        let start = Instant::now();
        let index = self.inner.load_peer_lessons(peer);
        eprintln!(
            "Storage: load_peer_lessons '{}' {:.1}ms",
            peer,
            start.elapsed().as_secs_f64() * 1000.0
        );
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

    fn dir(&self) -> &Path {
        self.inner.dir()
    }
}
