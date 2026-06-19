pub mod auth_token;
pub mod awareness;
pub mod bayes;
pub mod bookmark;
pub mod briefing;
pub mod classifier;
pub mod diag;
pub mod identity;
pub mod instrumented_storage;
pub mod lessons;
pub mod log;
pub mod message;
pub mod precommit;
pub mod reactions;
pub mod recall;
pub mod routing_identity;
pub mod search;
pub mod speech_guard;
pub mod storage;
#[cfg(test)]
mod storage_tests;
pub mod terminal_inject;
pub mod tokens;
pub mod usage;
pub mod velocity;
pub mod verdicts;

#[cfg(test)]
mod team_roster_tests;

pub use classifier::{MessageClass, classify};
pub use log::{
    BROADCAST_CHANNEL_CAPACITY, DirSession, LogError, Session, append_message, create_session,
    default_sessions_base, read_messages, read_or_create_dir_session, resume_or_create_session,
};
pub use message::{Message, MessageType};
pub use search::{SearchFilters, SearchResult, search};

#[cfg(test)]
mod auth_token_tests;
#[cfg(test)]
mod awareness_tests;
#[cfg(test)]
mod bayes_tests;
#[cfg(test)]
mod classifier_tests;
#[cfg(test)]
mod diag_tests;
#[cfg(test)]
mod lessons_tests;
#[cfg(test)]
mod log_tests;
#[cfg(test)]
mod message_tests;
#[cfg(test)]
mod precommit_tests;
#[cfg(test)]
mod recall_tests;
#[cfg(test)]
mod speech_guard_tests;
#[cfg(test)]
mod tokens_tests;
#[cfg(test)]
mod usage_tests;
#[cfg(test)]
mod velocity_tests;
