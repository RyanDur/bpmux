//! Awareness system — monitors the session and provides nudges.
//!
//! Five independent detectors, each with its own state:
//! - Experience: buffers teaching moments, nudges after pauses
//! - Recall: surfaces relevant lessons when topics match
//! - Pileon: detects when 3+ agents respond simultaneously
//! - Handoff: tracks explicit handoffs and follows up if unacknowledged
//! - Agents: tracks who is connected

pub mod agents;
pub mod experience;
pub mod handoff;
pub mod pileon;
pub mod recall;

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::session::Session;

use agents::AgentTracker;
use experience::ExperienceDetector;
use handoff::HandoffTracker;
use pileon::PileonDetector;
use recall::RecallSystem;

/// Spawn the awareness task that monitors the session broadcast.
pub fn spawn(session: Arc<Session>, owner_name: String) {
    let mut rx = session.subscribe();
    let messages = session.read_messages().unwrap_or_default();

    tokio::spawn(async move {
        let mut agents = AgentTracker::new(&owner_name);
        let mut experience = ExperienceDetector::new(&messages, &owner_name);
        let mut recall = RecallSystem::new(&session.dir);
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
                            recall.on_message(&session, &msg, sender, &owner_name);
                            pileon.on_message(&session, &msg, sender, &agents);
                            handoff.on_message(&msg, sender, &agents, &owner_name);
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = tokio::time::sleep(experience_timeout), if experience.has_pending() => {
                    experience.flush(&session, &agents);
                }
                _ = async {
                    if let Some(deadline) = handoff.deadline() {
                        tokio::time::sleep_until(deadline).await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                }, if handoff.is_pending() => {
                    handoff.timeout_nudge(&session);
                }
            }
        }
    });
}
