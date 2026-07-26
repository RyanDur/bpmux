use std::collections::HashSet;
use std::sync::Arc;

use awareness::{AwarenessHost, SurfacedTopicsRepo};
use hub::Hub;
use memory::lessons::LessonRepo;
use message::Message;
use message_log::LogError;
use tokio::sync::broadcast;

pub struct HubAwarenessHost {
    pub hub: Arc<Hub>,
    pub topics: Arc<dyn SurfacedTopicsRepo>,
    pub lessons: Arc<dyn LessonRepo>,
}

impl AwarenessHost for HubAwarenessHost {
    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        self.hub.read_recent(limit, offset, matches)
    }

    fn subscribe(&self) -> broadcast::Receiver<Message> {
        self.hub.subscribe()
    }

    fn broadcast(&self, msg: &Message) -> Result<(), LogError> {
        self.hub.broadcast(msg)
    }

    fn load_surfaced_topics(&self) -> HashSet<String> {
        self.topics.load_surfaced_topics()
    }

    fn save_surfaced_topics(&self, topics: &HashSet<String>) -> Result<(), std::io::Error> {
        self.topics.save_surfaced_topics(topics)
    }

    fn load_lessons(&self) -> memory::lessons::LessonIndex {
        self.lessons.load_lessons()
    }

    fn own_lesson_index(&self) -> memory::lessons::LessonIndex {
        memory::lessons::load_index(&self.hub.dir)
    }
}
