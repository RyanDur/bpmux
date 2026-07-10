mod file_log;
mod reverse_line_reader;

pub use file_log::{LogError, append_message, log_path, read_messages, read_recent};

pub trait MessageLog: Send + Sync {
    fn append_message(&self, msg: &message::Message) -> Result<(), LogError>;
    fn read_messages(&self) -> Result<Vec<message::Message>, LogError>;
    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&message::Message) -> bool,
    ) -> Result<Vec<message::Message>, LogError>;
    fn clear_messages(&self) -> Result<(), std::io::Error>;
}

#[cfg(test)]
mod file_log_format;
