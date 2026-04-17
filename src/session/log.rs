use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;
use tokio::sync::broadcast;

use crate::session::instrumented_storage::InstrumentedStorage;
use crate::session::message::Message;
use crate::session::storage::{FileStorage, Storage};

pub struct Session {
    pub dir: PathBuf,
    pub id: String,
    pub owner_name: String,
    pub wake_tx: broadcast::Sender<Message>,
    pub connected_peers: AtomicUsize,
    pub storage: Arc<dyn Storage>,
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            dir: self.dir.clone(),
            id: self.id.clone(),
            owner_name: self.owner_name.clone(),
            wake_tx: self.wake_tx.clone(),
            connected_peers: AtomicUsize::new(self.connected_peers.load(Ordering::Relaxed)),
            storage: self.storage.clone(),
        }
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("dir", &self.dir)
            .field("id", &self.id)
            .field("owner_name", &self.owner_name)
            .finish()
    }
}

impl Session {
    pub fn new(dir: PathBuf, owner_name: String, wake_tx: broadcast::Sender<Message>) -> Self {
        let id = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let storage: Arc<dyn Storage> =
            Arc::new(InstrumentedStorage::new(FileStorage::new(dir.clone())));
        Self {
            dir,
            id,
            owner_name,
            wake_tx,
            connected_peers: AtomicUsize::new(0),
            storage,
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
        self.storage.append_message(msg)?;
        let _ = self.wake_tx.send(msg.clone());
        Ok(())
    }

    pub fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        self.storage.read_messages()
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
