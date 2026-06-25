use std::path::{Path, PathBuf};
use thiserror::Error;

use super::reverse_line_reader::ReverseLineReader;
use crate::message::Message;

#[derive(Error, Debug)]
pub enum LogError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn log_path(session_dir: &Path) -> PathBuf {
    session_dir.join("messages-current.jsonl")
}

pub fn append_message(session_dir: &Path, message: &Message) -> Result<(), LogError> {
    let path = log_path(session_dir);
    let mut line = serde_json::to_string(message)?;
    line.push('\n');
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    file.flush()?;
    Ok(())
}

pub fn read_messages(session_dir: &Path) -> Result<Vec<Message>, LogError> {
    let path = log_path(session_dir);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)?;
    let messages: Result<Vec<Message>, _> = content
        .lines()
        .filter(|line| !line.is_empty())
        .map(serde_json::from_str)
        .collect();
    Ok(messages?)
}

pub fn read_recent(
    session_dir: &Path,
    limit: usize,
    offset: usize,
    matches: &dyn Fn(&Message) -> bool,
) -> Result<Vec<Message>, LogError> {
    let want = limit.saturating_add(offset);
    let path = log_path(session_dir);
    if want == 0 || !path.exists() {
        return Ok(vec![]);
    }
    let mut lines = ReverseLineReader::open(&path)?;
    let mut newest_first = Vec::with_capacity(want);
    while newest_first.len() < want {
        let Some(line) = lines.next_line()? else {
            break;
        };
        let message: Message = serde_json::from_str(&line)?;
        if matches(&message) {
            newest_first.push(message);
        }
    }
    Ok(newest_first.into_iter().skip(offset).take(limit).collect())
}
