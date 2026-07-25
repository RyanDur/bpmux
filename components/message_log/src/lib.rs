mod file_log;
mod reverse_line_reader;

pub const DEFAULT_MESSAGE_LIMIT: usize = 20;

pub use file_log::{
    LogError, append_message, clear_messages, log_path, read_cluster, read_messages,
    read_messages_since, read_recent, rotate_if_larger_than, search_hits,
};

pub trait MessageLog: Send + Sync {
    fn append_message(&self, msg: &message::Message) -> Result<(), LogError>;
    fn read_messages(&self) -> Result<Vec<message::Message>, LogError>;
    fn search_hits(&self, needle_lower: &str) -> Result<Vec<u64>, LogError>;
    fn read_cluster(
        &self,
        center: u64,
        radius: u64,
    ) -> Result<Vec<(u64, message::Message)>, LogError>;
    fn read_messages_since(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<message::Message>, LogError>;
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

#[cfg(test)]
mod log_tests;
