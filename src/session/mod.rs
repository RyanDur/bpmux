pub mod log;
pub mod message;

pub use log::{append_message, read_messages, LogError};
pub use message::Message;
