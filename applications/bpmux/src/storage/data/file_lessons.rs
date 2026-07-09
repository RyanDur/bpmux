use super::FileStorage;
use crate::memory::lessons::LessonIndex;

impl crate::memory::lessons::LessonRepo for FileStorage {
    fn load_lessons(&self) -> LessonIndex {
        crate::memory::lessons::load_index(&self.dir)
    }

    fn add_lesson(
        &self,
        topic: &str,
        lesson: &str,
        author_role: &str,
        evidence: Vec<String>,
    ) -> Result<(), std::io::Error> {
        crate::memory::lessons::add_lesson(&self.dir, topic, lesson, author_role, evidence)
    }

    fn load_peer_lessons(&self, peer: &str) -> LessonIndex {
        crate::memory::lessons::load_peer_index(&self.dir, peer)
    }

    fn save_peer_lessons(&self, peer: &str, index: &LessonIndex) -> Result<(), std::io::Error> {
        crate::memory::lessons::save_peer_index(&self.dir, peer, index)
    }

    fn list_peers(&self) -> Vec<String> {
        crate::memory::lessons::list_peer_names(&self.dir)
    }
}
