use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::broadcast;

use crate::log::LogError;
use crate::message::Message;
use crate::storage::{Storage, StorageBackend};

pub const BROADCAST_CHANNEL_CAPACITY: usize = 10_000;

pub struct Hub {
    pub dir: PathBuf,
    pub id: String,
    pub owner_name: String,
    pub wake_tx: broadcast::Sender<Message>,
    pub connected_peers: AtomicUsize,
    pub storage: Arc<dyn Storage>,
    pub roster: Mutex<HashSet<String>>,
}

impl Clone for Hub {
    fn clone(&self) -> Self {
        Self {
            dir: self.dir.clone(),
            id: self.id.clone(),
            owner_name: self.owner_name.clone(),
            wake_tx: self.wake_tx.clone(),
            connected_peers: AtomicUsize::new(self.connected_peers.load(Ordering::Relaxed)),
            storage: self.storage.clone(),
            roster: Mutex::new(HashSet::new()),
        }
    }
}

impl std::fmt::Debug for Hub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hub")
            .field("dir", &self.dir)
            .field("id", &self.id)
            .field("owner_name", &self.owner_name)
            .finish()
    }
}

impl Hub {
    pub fn new(dir: PathBuf, owner_name: String, wake_tx: broadcast::Sender<Message>) -> Self {
        Self::with_backend(dir, owner_name, wake_tx, StorageBackend::resolve())
    }

    pub fn with_backend(
        dir: PathBuf,
        owner_name: String,
        wake_tx: broadcast::Sender<Message>,
        backend: StorageBackend,
    ) -> Self {
        let id = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let storage = backend.build(&dir);
        let roster = load_seed_roster(&dir);
        Self {
            dir,
            id,
            owner_name,
            wake_tx,
            connected_peers: AtomicUsize::new(0),
            storage,
            roster: Mutex::new(roster),
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

    pub fn roster_join(&self, identity: &str) -> usize {
        let mut r = self.roster.lock().unwrap_or_else(|e| e.into_inner());
        r.insert(identity.to_string());
        r.len()
    }

    pub fn roster_leave(&self, identity: &str) -> usize {
        let mut r = self.roster.lock().unwrap_or_else(|e| e.into_inner());
        r.remove(identity);
        r.len()
    }

    pub fn roster_snapshot(&self) -> Vec<String> {
        let r = self.roster.lock().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<String> = r.iter().cloned().collect();
        out.sort();
        out
    }

    pub fn reconcile_roster(&self, live_terminal_ids: &[String]) -> Vec<String> {
        use crate::routing::bridge_agent_name;
        const AUTO_AGENT_PREFIX: &str = "agent-";
        let live_auto_names: HashSet<String> = live_terminal_ids
            .iter()
            .map(|terminal_id| bridge_agent_name(terminal_id))
            .collect();
        let mut r = self.roster.lock().unwrap_or_else(|e| e.into_inner());
        let ghosts: Vec<String> = r
            .iter()
            .filter(|identity| {
                let short = identity.split(':').next().unwrap_or(identity);
                short.starts_with(AUTO_AGENT_PREFIX) && !live_auto_names.contains(short)
            })
            .cloned()
            .collect();
        for ghost in &ghosts {
            r.remove(ghost);
        }
        ghosts
    }

    pub fn broadcast(&self, msg: &Message) -> Result<(), LogError> {
        let mut msg = msg.clone();
        msg.ensure_id();
        self.storage.append_message(&msg)?;
        let _ = self.wake_tx.send(msg);
        Ok(())
    }

    pub fn broadcast_ephemeral(&self, msg: &Message) {
        let _ = self.wake_tx.send(msg.clone());
    }

    pub fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        self.storage.read_messages()
    }

    pub fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        self.storage.read_recent(limit, offset, matches)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Message> {
        self.wake_tx.subscribe()
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

fn load_seed_roster(dir: &Path) -> HashSet<String> {
    let path = dir.join("roster_seed.json");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return HashSet::new();
    };
    let parsed: Result<Vec<String>, _> = serde_json::from_str(&content);
    let Ok(list) = parsed else {
        return HashSet::new();
    };
    list.into_iter().collect()
}
