use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;
use tokio::sync::broadcast;

use crate::session::message::Message;

#[derive(Debug)]
pub struct Session {
    pub dir: PathBuf,
    pub id: String,
    pub wake_tx: broadcast::Sender<Message>,
    pub connected_peers: AtomicUsize,
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            dir: self.dir.clone(),
            id: self.id.clone(),
            wake_tx: self.wake_tx.clone(),
            connected_peers: AtomicUsize::new(self.connected_peers.load(Ordering::Relaxed)),
        }
    }
}

impl Session {
    pub fn new(dir: PathBuf, wake_tx: broadcast::Sender<Message>) -> Self {
        let id = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        Self {
            dir,
            id,
            wake_tx,
            connected_peers: AtomicUsize::new(0),
        }
    }

    pub fn peer_connected(&self) -> usize {
        self.connected_peers.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn peer_disconnected(&self) -> usize {
        self.connected_peers
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                Some(n.saturating_sub(1))
            })
            .unwrap_or(0)
            .saturating_sub(1)
    }

    pub fn peer_count(&self) -> usize {
        self.connected_peers.load(Ordering::Relaxed)
    }

    pub fn broadcast(&self, msg: &Message) -> Result<(), LogError> {
        append_message(&self.dir, msg)?;
        let _ = self.wake_tx.send(msg.clone());
        Ok(())
    }

    pub fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        read_messages(&self.dir)
    }

    pub fn read_pins(&self) -> Result<Vec<String>, LogError> {
        read_pins(&self.dir)
    }

    pub fn add_pin(&self, text: String) -> Result<Result<Vec<String>, String>, LogError> {
        add_pin(&self.dir, text)
    }

    pub fn remove_pin(&self, index: usize) -> Result<Result<Vec<String>, String>, LogError> {
        remove_pin(&self.dir, index)
    }

    pub fn save_handoff(&self, name: &str, text: &str) -> Result<(), LogError> {
        save_handoff(&self.dir, name, text)
    }

    pub fn read_handoff(&self, name: &str) -> Result<Option<String>, LogError> {
        read_handoff(&self.dir, name)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Message> {
        self.wake_tx.subscribe()
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

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

// --- Pin persistence ---

const MAX_PINS: usize = 5;

fn pins_path(session_dir: &Path) -> PathBuf {
    session_dir.join("pins.json")
}

pub fn read_pins(session_dir: &Path) -> Result<Vec<String>, LogError> {
    let path = pins_path(session_dir);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)?;
    let pins: Vec<String> = serde_json::from_str(&content)?;
    Ok(pins)
}

pub fn write_pins(session_dir: &Path, pins: &[String]) -> Result<(), LogError> {
    let path = pins_path(session_dir);
    let json = serde_json::to_string(pins)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn add_pin(
    session_dir: &Path,
    text: String,
) -> Result<Result<Vec<String>, String>, LogError> {
    let mut pins = read_pins(session_dir)?;
    // Duplicate guard: case-insensitive exact match
    if let Some(pos) = pins
        .iter()
        .position(|p| p.eq_ignore_ascii_case(&text))
    {
        return Ok(Err(format!("already pinned as #{}", pos + 1)));
    }
    if pins.len() >= MAX_PINS {
        return Ok(Err("max 5 pins — unpin one first".to_string()));
    }
    pins.push(text);
    write_pins(session_dir, &pins)?;
    Ok(Ok(pins))
}

pub fn remove_pin(
    session_dir: &Path,
    index: usize,
) -> Result<Result<Vec<String>, String>, LogError> {
    let mut pins = read_pins(session_dir)?;
    if index == 0 || index > pins.len() {
        return Ok(Err(format!("no pin #{index}. Type /pins to list.")));
    }
    pins.remove(index - 1);
    write_pins(session_dir, &pins)?;
    Ok(Ok(pins))
}

// --- Handoff persistence ---

const MAX_HANDOFF_CHARS: usize = 500;

fn handoff_path(session_dir: &Path, name: &str) -> PathBuf {
    session_dir.join(format!("handoff_{name}.txt"))
}

pub fn save_handoff(session_dir: &Path, name: &str, text: &str) -> Result<(), LogError> {
    let truncated: String = text.chars().take(MAX_HANDOFF_CHARS).collect();
    let path = handoff_path(session_dir, name);
    std::fs::write(path, truncated)?;
    Ok(())
}

pub fn read_handoff(session_dir: &Path, name: &str) -> Result<Option<String>, LogError> {
    let path = handoff_path(session_dir, name);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    if content.is_empty() {
        return Ok(None);
    }
    Ok(Some(content))
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
            message_type: None,
            to: None,
        };
        let msg2 = Message {
            from: "ryan:human".to_string(),
            text: "second message".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
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
        let current = base.path().join("current");
        assert!(current.is_symlink());
    }

    #[test]
    fn resumes_existing_session() {
        let base = TempDir::new().expect("temp base");
        let dir1 = create_session(base.path()).expect("create session");
        assert!(dir1.exists());
        let msg = Message {
            from: "ryan:human".to_string(),
            text: "persisted message".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        };
        append_message(&dir1, &msg).expect("append");
        let dir2 = resume_or_create_session(base.path()).expect("resume");
        assert_eq!(dir1, dir2);
        let messages = read_messages(&dir2).expect("read");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "persisted message");
    }

    #[test]
    fn session_broadcasts_and_reads() {
        let dir = TempDir::new().expect("temp dir");
        let (tx, mut rx) = broadcast::channel(16);
        let session = Session::new(dir.path().to_path_buf(), tx);
        let msg = Message {
            from: "ryan:human".to_string(),
            text: "via session".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        };
        session.broadcast(&msg).expect("broadcast");
        let received = rx.try_recv().expect("receive broadcast");
        assert_eq!(received.text, "via session");
        let messages = session.read_messages().expect("read");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "via session");
    }

    #[test]
    fn session_derives_id_from_dir_name() {
        let base = TempDir::new().expect("temp base");
        let session_dir = base.path().join("my-session-id");
        std::fs::create_dir_all(&session_dir).expect("create dir");
        let (tx, _rx) = broadcast::channel(16);
        let session = Session::new(session_dir, tx);
        assert_eq!(session.id, "my-session-id");
    }

    #[test]
    fn saves_and_reads_handoff() {
        let dir = TempDir::new().expect("temp dir");
        save_handoff(
            dir.path(),
            "dev",
            "Working on auth refactor. Tests passing.",
        )
        .expect("save");
        let handoff = read_handoff(dir.path(), "dev").expect("read");
        assert_eq!(
            handoff,
            Some("Working on auth refactor. Tests passing.".to_string())
        );
    }

    #[test]
    fn handoff_truncates_at_500_chars() {
        let dir = TempDir::new().expect("temp dir");
        let long_text: String = "x".repeat(600);
        save_handoff(dir.path(), "dev", &long_text).expect("save");
        let handoff = read_handoff(dir.path(), "dev")
            .expect("read")
            .expect("some");
        assert_eq!(handoff.len(), 500);
    }

    #[test]
    fn handoff_returns_none_when_missing() {
        let dir = TempDir::new().expect("temp dir");
        let handoff = read_handoff(dir.path(), "dev").expect("read");
        assert_eq!(handoff, None);
    }

    #[test]
    fn handoff_overwrites_previous() {
        let dir = TempDir::new().expect("temp dir");
        save_handoff(dir.path(), "dev", "first").expect("save 1");
        save_handoff(dir.path(), "dev", "second").expect("save 2");
        let handoff = read_handoff(dir.path(), "dev").expect("read");
        assert_eq!(handoff, Some("second".to_string()));
    }

    #[test]
    fn handoff_is_per_participant() {
        let dir = TempDir::new().expect("temp dir");
        save_handoff(dir.path(), "dev", "dev context").expect("save dev");
        save_handoff(dir.path(), "pm", "pm context").expect("save pm");
        assert_eq!(
            read_handoff(dir.path(), "dev").expect("read"),
            Some("dev context".to_string())
        );
        assert_eq!(
            read_handoff(dir.path(), "pm").expect("read"),
            Some("pm context".to_string())
        );
    }

    #[test]
    fn pin_duplicate_guard_rejects_case_insensitive() {
        let dir = TempDir::new().expect("temp dir");
        let result = add_pin(dir.path(), "channels-as-views".to_string()).expect("io");
        assert!(result.is_ok());
        let result = add_pin(dir.path(), "Channels-As-Views".to_string()).expect("io");
        assert_eq!(result.unwrap_err(), "already pinned as #1");
    }

    #[test]
    fn pin_duplicate_guard_allows_different_text() {
        let dir = TempDir::new().expect("temp dir");
        add_pin(dir.path(), "first pin".to_string())
            .expect("io")
            .expect("ok");
        let result = add_pin(dir.path(), "second pin".to_string())
            .expect("io")
            .expect("ok");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn peer_count_shared_through_arc() {
        let dir = TempDir::new().expect("temp dir");
        let (tx, _rx) = broadcast::channel(16);
        let session = std::sync::Arc::new(Session::new(dir.path().to_path_buf(), tx));
        let server_session = std::sync::Arc::clone(&session);

        assert_eq!(session.peer_count(), 0);

        server_session.peer_connected();
        assert_eq!(session.peer_count(), 1);

        server_session.peer_connected();
        assert_eq!(session.peer_count(), 2);

        server_session.peer_disconnected();
        assert_eq!(session.peer_count(), 1);

        server_session.peer_disconnected();
        assert_eq!(session.peer_count(), 0);
    }
}
