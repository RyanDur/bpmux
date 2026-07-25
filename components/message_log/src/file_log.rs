use std::path::{Path, PathBuf};
use thiserror::Error;

use super::reverse_line_reader::ReverseLineReader;
use message::Message;

#[derive(Error, Debug)]
pub enum LogError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn log_path(data_dir: &Path) -> PathBuf {
    data_dir.join("messages-current.jsonl")
}

pub fn append_message(data_dir: &Path, message: &Message) -> Result<(), LogError> {
    let path = log_path(data_dir);
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

pub fn read_messages(data_dir: &Path) -> Result<Vec<Message>, LogError> {
    let path = log_path(data_dir);
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

pub fn clear_messages(data_dir: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_file(log_path(data_dir)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn rotate_if_larger_than(data_dir: &Path, max_bytes: u64) -> Result<bool, std::io::Error> {
    let path = log_path(data_dir);
    let size = match std::fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    if size <= max_bytes {
        return Ok(false);
    }
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%3f");
    let archive = data_dir.join(format!("messages-archived-{stamp}.jsonl"));
    std::fs::rename(&path, archive)?;
    Ok(true)
}

pub fn read_messages_since(
    data_dir: &Path,
    cutoff: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<Message>, LogError> {
    let path = log_path(data_dir);
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut recent = Vec::new();
    for line in std::fs::read_to_string(&path)?.lines() {
        if line.is_empty() {
            continue;
        }
        let msg: Message = serde_json::from_str(line)?;
        if msg.ts >= cutoff {
            recent.push(msg);
        }
    }
    Ok(recent)
}

pub fn read_recent(
    data_dir: &Path,
    limit: usize,
    offset: usize,
    matches: &dyn Fn(&Message) -> bool,
) -> Result<Vec<Message>, LogError> {
    let want = limit.saturating_add(offset);
    let path = log_path(data_dir);
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
