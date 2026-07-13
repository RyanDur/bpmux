use std::collections::HashSet;
use tokio::sync::broadcast;

use super::Hub;
use crate::awareness::AwarenessHost;
use message::Message;
use message_log::LogError;

impl AwarenessHost for Hub {
    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        self.read_recent(limit, offset, matches)
    }

    fn subscribe(&self) -> broadcast::Receiver<Message> {
        self.subscribe()
    }

    fn broadcast(&self, msg: &Message) -> Result<(), LogError> {
        self.broadcast(msg)
    }

    fn load_surfaced_topics(&self) -> HashSet<String> {
        self.storage.load_surfaced_topics()
    }

    fn save_surfaced_topics(&self, topics: &HashSet<String>) -> Result<(), std::io::Error> {
        self.storage.save_surfaced_topics(topics)
    }

    fn load_lessons(&self) -> memory::lessons::LessonIndex {
        self.storage.load_lessons()
    }

    fn own_lesson_index(&self) -> memory::lessons::LessonIndex {
        memory::lessons::load_index(&self.dir)
    }
}
