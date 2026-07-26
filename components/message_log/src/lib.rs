pub mod data_gateway;
pub mod db;
mod file_log;
pub mod import;
mod reverse_line_reader;

pub const DEFAULT_MESSAGE_LIMIT: usize = 20;

pub use file_log::{
    LogError, append_message, clear_messages, log_path, read_cluster, read_messages,
    read_messages_since, read_recent, rotate_if_larger_than, search_hits,
};

pub fn drive_search(
    hits: Vec<u64>,
    radius: u64,
    read_cluster: impl Fn(u64, u64) -> Result<Vec<(u64, message::Message)>, LogError>,
    accept: &mut dyn FnMut(&[message::Message], usize) -> bool,
) -> Result<(), LogError> {
    let mut used_through: Option<u64> = None;
    for hit in hits {
        if used_through.is_some_and(|through| hit <= through) {
            continue;
        }
        let cluster = read_cluster(hit, radius)?;
        let Some(center_index) = cluster.iter().position(|(ordinal, _)| *ordinal == hit) else {
            continue;
        };
        let messages: Vec<message::Message> = cluster.into_iter().map(|(_, m)| m).collect();
        if accept(&messages, center_index) {
            used_through = Some(hit + radius);
        }
    }
    Ok(())
}

pub trait MessageLog: Send + Sync {
    fn append_message(&self, msg: &message::Message) -> Result<(), LogError>;
    fn read_messages(&self) -> Result<Vec<message::Message>, LogError>;
    fn search_conversations(
        &self,
        needle_lower: &str,
        addressee: Option<&str>,
        radius: u64,
        accept: &mut dyn FnMut(&[message::Message], usize) -> bool,
    ) -> Result<(), LogError>;
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
mod import_tests;
#[cfg(test)]
mod log_tests;
