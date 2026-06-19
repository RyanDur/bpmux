use std::sync::Arc;
use tokio::sync::mpsc;

pub struct RemotePeer {
    pub name: String,
    pub(super) outbound_tx: mpsc::Sender<String>,
    pub(super) connected: Arc<std::sync::atomic::AtomicBool>,
}

impl RemotePeer {
    pub fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn send_json(&self, json: &str) -> Result<(), PeerError> {
        self.outbound_tx
            .try_send(json.to_string())
            .map_err(|_| PeerError::Disconnected)
    }
}

#[derive(Debug)]
pub enum PeerError {
    Disconnected,
    Serialization(String),
    Log(String),
}

impl std::fmt::Display for PeerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "peer disconnected"),
            Self::Serialization(e) => write!(f, "serialization error: {e}"),
            Self::Log(e) => write!(f, "log error: {e}"),
        }
    }
}

impl std::error::Error for PeerError {}
