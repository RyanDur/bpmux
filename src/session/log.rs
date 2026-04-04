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
        };
        let msg2 = Message {
            from: "ryan:human".to_string(),
            text: "second message".to_string(),
            ts: Utc::now(),
            message_type: None,
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
