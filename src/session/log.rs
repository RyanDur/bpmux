use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::session::message::Message;

pub fn default_sessions_base() -> Result<PathBuf, LogError> {
    let base = dirs::home_dir()
        .ok_or_else(|| {
            LogError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "home directory not found",
            ))
        })?
        .join(".bpmux")
        .join("sessions");
    Ok(base)
}

pub fn resume_or_create_session(base: &Path) -> Result<PathBuf, LogError> {
    let current = base.join("current");

    // If a current symlink exists and points to a valid directory, resume it
    if current.is_symlink() {
        let target = std::fs::read_link(&current)?;
        if target.is_dir() {
            return Ok(target);
        }
    }

    create_session(base)
}

pub fn create_session(base: &Path) -> Result<PathBuf, LogError> {
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
        let base = TempDir::new().expect("temp base");
        let dir = create_session(base.path()).expect("create session");
        assert!(dir.exists());
        assert!(dir.is_dir());
        // current symlink should point to the session
        let current = base.path().join("current");
        assert!(current.is_symlink());
    }

    #[test]
    fn resumes_existing_session() {
        let base = TempDir::new().expect("temp base");

        // Create a session first
        let dir1 = create_session(base.path()).expect("create session");
        assert!(dir1.exists());

        // Write a message to it
        let msg = Message {
            from: "ryan:human".to_string(),
            text: "persisted message".to_string(),
            ts: Utc::now(),
        };
        append_message(&dir1, &msg).expect("append");

        // Resume should return the same directory
        let dir2 = resume_or_create_session(base.path()).expect("resume");
        assert_eq!(dir1, dir2);

        // Messages should be readable from the resumed session
        let messages = read_messages(&dir2).expect("read");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "persisted message");
    }
}
