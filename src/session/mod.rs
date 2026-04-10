pub mod awareness;
pub mod bayes;
pub mod classifier;
pub mod diag;
pub mod instrumented_storage;
pub mod lessons;
pub mod log;
pub mod message;
pub mod recall;
pub mod search;
pub mod storage;

pub use classifier::{MessageClass, classify};
pub use log::{
    LogError, Session, append_message, create_session, default_sessions_base, read_messages,
    resume_or_create_session,
};
pub use message::{Message, MessageType};
pub use search::{SearchFilters, SearchResult, search};

#[cfg(test)]
mod awareness_tests;
#[cfg(test)]
mod classifier_tests;
#[cfg(test)]
mod lessons_tests;
#[cfg(test)]
mod log_tests;
#[cfg(test)]
mod message_tests;
#[cfg(test)]
mod recall_tests;
