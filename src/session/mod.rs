pub mod log;
pub mod message;
pub mod search;

pub use log::{
    LogError, Session, append_message, create_session, default_sessions_base, read_messages,
    resume_or_create_session,
};
pub use message::{Message, MessageType};
pub use search::{SearchFilters, SearchResult, search};

#[cfg(test)]
mod log_tests;
#[cfg(test)]
mod message_tests;
