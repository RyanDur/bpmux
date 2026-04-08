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

    pub fn add_pin(&self, text: String) -> Result<Result<Vec<String>, &'static str>, LogError> {
        add_pin(&self.dir, text)
    }

    pub fn remove_pin(&self, index: usize) -> Result<Result<Vec<String>, String>, LogError> {
        remove_pin(&self.dir, index)
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
) -> Result<Result<Vec<String>, &'static str>, LogError> {
    let mut pins = read_pins(session_dir)?;
    if pins.len() >= MAX_PINS {
        return Ok(Err("max 5 pins — /unpin one first"));
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
