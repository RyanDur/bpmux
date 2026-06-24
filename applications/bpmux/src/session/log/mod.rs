mod file_log;
mod reverse_line_reader;

pub use file_log::{LogError, append_message, log_path, read_messages, read_recent};
