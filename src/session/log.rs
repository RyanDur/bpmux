use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::session::message::Message;

pub fn create_session_dir() -> Result<PathBuf, LogError> {
    let base = dirs::home_dir()
        .ok_or_else(|| LogError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "home directory not found",
        )))?
        .join(".bpmux")
        .join("sessions");

    let session_id = uuid::Uuid::new_v4().to_string();
    let session_dir = base.join(&session_id);
    std::fs::create_dir_all(&session_dir)?;

    // Create/update "current" symlink
    let current = base.join("current");
    if current.exists() || current.is_symlink() {
        std::fs::remove_file(&current)?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&session_dir, &current)?;

    Ok(session_dir)
}

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[test]
    fn appends_and_reads_messages() {
        let dir = TempDir::new().expect("temp dir");

        let msg1 = Message {
            from: "ryan:human".to_string(),
            text: "first message".to_string(),
            ts: Utc::now(),
        };
        let msg2 = Message {
            from: "ryan:human".to_string(),
            text: "second message".to_string(),
            ts: Utc::now(),
        };

        append_message(dir.path(), &msg1).expect("append 1");
        append_message(dir.path(), &msg2).expect("append 2");

        let messages = read_messages(dir.path()).expect("read");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "first message");
        assert_eq!(messages[1].text, "second message");
    }

    #[test]
    fn reads_empty_when_no_log_exists() {
        let dir = TempDir::new().expect("temp dir");
        let messages = read_messages(dir.path()).expect("read");
        assert!(messages.is_empty());
    }

    #[test]
    fn creates_session_directory() {
        let dir = create_session_dir().expect("create session");
        assert!(dir.exists());
        assert!(dir.is_dir());

        // Clean up
        std::fs::remove_dir_all(dir.parent().expect("parent").parent().expect("base"))
            .expect("cleanup");
    }
}
