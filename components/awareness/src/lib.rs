pub mod agents;
pub mod data_gateway;
pub mod db;
pub mod experience;
pub mod handoff;
pub mod pileon;
pub mod recall;

use std::sync::Arc;
use tokio::sync::broadcast;

use agents::AgentTracker;
use experience::ExperienceDetector;
use handoff::HandoffTracker;
use pileon::PileonDetector;
use recall::RecallSystem;

pub trait SurfacedTopicsRepo: Send + Sync {
    fn load_surfaced_topics(&self) -> std::collections::HashSet<String>;
    fn save_surfaced_topics(
        &self,
        topics: &std::collections::HashSet<String>,
    ) -> Result<(), std::io::Error>;
}

pub trait AwarenessHost: Send + Sync {
    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&message::Message) -> bool,
    ) -> Result<Vec<message::Message>, message_log::LogError>;
    fn subscribe(&self) -> broadcast::Receiver<message::Message>;
    fn broadcast(&self, msg: &message::Message) -> Result<(), message_log::LogError>;
    fn load_surfaced_topics(&self) -> std::collections::HashSet<String>;
    fn save_surfaced_topics(
        &self,
        topics: &std::collections::HashSet<String>,
    ) -> Result<(), std::io::Error>;
    fn load_lessons(&self) -> memory::lessons::LessonIndex;
    fn own_lesson_index(&self) -> memory::lessons::LessonIndex;
}

const TRAIN_WINDOW: usize = 2000;

fn training_messages(host: &dyn AwarenessHost) -> Vec<message::Message> {
    host.read_recent(TRAIN_WINDOW, 0, &|_| true)
        .unwrap_or_default()
}

pub fn spawn(host: Arc<dyn AwarenessHost>, owner_name: String) {
    let mut rx = host.subscribe();
    let messages = training_messages(&*host);

    tokio::spawn(async move {
        let mut agents = AgentTracker::new(&owner_name);
        let mut experience = ExperienceDetector::new(&messages, &owner_name);
        let mut recall = RecallSystem::new(&*host);
        let mut pileon = PileonDetector::new(&owner_name);
        let mut handoff = HandoffTracker::new();

        loop {
            let experience_timeout = experience.timeout();

            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            let sender = msg.from.split(':').next().unwrap_or(&msg.from);
                            agents.on_message(&msg);
                            experience.on_message(&msg, sender);
                            recall.on_message(&*host, &msg, sender, &owner_name);
                            pileon.on_message(&*host, &msg, sender, &agents);
                            handoff.on_message(&msg, sender, &agents, &owner_name);
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = tokio::time::sleep(experience_timeout), if experience.has_pending() => {
                    experience.flush(&*host, &agents);
                }
                _ = async {
                    if let Some(deadline) = handoff.deadline() {
                        tokio::time::sleep_until(deadline).await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                }, if handoff.is_pending() => {
                    handoff.timeout_nudge(&*host);
                }
            }
        }
    });
}

#[cfg(test)]
mod awareness_tests;
