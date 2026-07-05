use std::time::Instant;

use super::InstrumentedStorage;
use crate::memory::lessons::LessonIndex;
use crate::storage::Storage;

impl<S: Storage> crate::memory::lessons::LessonRepo for InstrumentedStorage<S> {
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
}
