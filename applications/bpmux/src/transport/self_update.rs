use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use tokio_util::sync::CancellationToken;

pub const RESTART_EXIT_CODE: i32 = 200;

const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;

#[derive(Debug, Clone, Default)]
pub struct RestartSignal {
    flag: Arc<AtomicBool>,
}

impl RestartSignal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fire(&self) {
        self.flag.store(true, Ordering::Release);
    }

    pub fn was_fired(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

pub fn spawn_binary_watcher(
    ct: CancellationToken,
    signal: RestartSignal,
) -> tokio::task::JoinHandle<()> {
    let interval_secs = std::env::var("BPMUX_BINARY_WATCH_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_POLL_INTERVAL_SECS);
    let path = match resolve_watch_path() {
        Ok(p) => p,
        Err(e) => {
            diag::log_diag(&format!(
                "self-update: cannot resolve watch path ({e}); watcher disabled"
            ));
            return tokio::spawn(async {});
        }
    };
    spawn_binary_watcher_at(path, ct, signal, Duration::from_secs(interval_secs))
}

fn resolve_watch_path() -> Result<PathBuf, String> {
    let override_path = std::env::var("BPMUX_BINARY_WATCH_PATH")
        .ok()
        .filter(|s| !s.is_empty());
    resolve_watch_path_with_override(override_path)
}

pub(super) fn resolve_watch_path_with_override(
    env_value: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(override_path) = env_value {
        return Ok(PathBuf::from(override_path));
    }
    std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))
}

pub fn spawn_binary_watcher_at(
    path: PathBuf,
    ct: CancellationToken,
    signal: RestartSignal,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let initial_mtime = match read_mtime(&path) {
            Some(t) => t,
            None => {
                diag::log_diag(&format!(
                    "self-update: cannot stat binary {}; watcher disabled",
                    path.display()
                ));
                return;
            }
        };
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = ct.cancelled() => return,
                _ = ticker.tick() => {
                    if let Some(now_mtime) = read_mtime(&path)
                        && now_mtime != initial_mtime
                    {
                        diag::log_diag(&format!(
                            "self-update: binary mtime changed at {}; firing graceful shutdown for restart",
                            path.display()
                        ));
                        signal.fire();
                        ct.cancel();
                        return;
                    }
                }
            }
        }
    })
}

fn read_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RestartHistory {
    pub recent_restarts_at: Vec<chrono::DateTime<chrono::Utc>>,
}

impl RestartHistory {
    pub fn record(&mut self, now: chrono::DateTime<chrono::Utc>) {
        self.recent_restarts_at.push(now);
    }

    pub fn prune_older_than(&mut self, cutoff: chrono::DateTime<chrono::Utc>) {
        self.recent_restarts_at.retain(|t| *t >= cutoff);
    }

    pub fn count_in_window(&self, window_start: chrono::DateTime<chrono::Utc>) -> usize {
        self.recent_restarts_at
            .iter()
            .filter(|t| **t >= window_start)
            .count()
    }
}

pub fn loop_guard_should_halt(
    history: &RestartHistory,
    now: chrono::DateTime<chrono::Utc>,
    window: Duration,
    max_restarts: usize,
) -> bool {
    let window_start = now - chrono::Duration::from_std(window).unwrap_or_default();
    history.count_in_window(window_start) >= max_restarts
}

pub fn restart_state_path() -> Result<PathBuf, std::io::Error> {
    if let Ok(p) = std::env::var("BPMUX_RESTART_STATE_PATH")
        && !p.is_empty()
    {
        return Ok(PathBuf::from(p));
    }
    let base = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(base.join(".bpmux").join("restart-state.json"))
}

pub fn load_restart_history(path: &Path) -> RestartHistory {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return RestartHistory::default(),
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save_restart_history(path: &Path, history: &RestartHistory) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(history)?;
    std::fs::write(path, bytes)
}
