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
            crate::session::diag::log_diag(&format!(
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

fn resolve_watch_path_with_override(env_value: Option<String>) -> Result<PathBuf, String> {
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
                crate::session::diag::log_diag(&format!(
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
                        crate::session::diag::log_diag(&format!(
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::fs;

    fn fixed_now() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 5, 5, 12, 0, 0).unwrap()
    }

    #[test]
    fn restart_signal_starts_unfired() {
        let s = RestartSignal::new();
        assert!(!s.was_fired());
    }

    #[test]
    fn restart_signal_fire_flips_to_true() {
        let s = RestartSignal::new();
        s.fire();
        assert!(s.was_fired());
    }

    #[test]
    fn restart_signal_clones_share_state() {
        let s = RestartSignal::new();
        let s2 = s.clone();
        s.fire();
        assert!(
            s2.was_fired(),
            "Arc-shared flag — clones see fire from any holder"
        );
    }

    #[test]
    fn restart_history_count_in_window_counts_only_within_window() {
        let mut h = RestartHistory::default();
        let now = fixed_now();
        h.record(now - chrono::Duration::seconds(120));
        h.record(now - chrono::Duration::seconds(30));
        h.record(now - chrono::Duration::seconds(10));
        let window_start = now - chrono::Duration::seconds(60);
        assert_eq!(h.count_in_window(window_start), 2);
    }

    #[test]
    fn restart_history_prune_drops_old_entries() {
        let mut h = RestartHistory::default();
        let now = fixed_now();
        h.record(now - chrono::Duration::seconds(120));
        h.record(now - chrono::Duration::seconds(30));
        h.prune_older_than(now - chrono::Duration::seconds(60));
        assert_eq!(h.recent_restarts_at.len(), 1);
    }

    #[test]
    fn loop_guard_halts_when_max_restarts_reached_in_window() {
        let mut h = RestartHistory::default();
        let now = fixed_now();
        for offset in [0, 5, 10, 15, 20] {
            h.record(now - chrono::Duration::seconds(offset));
        }
        assert!(loop_guard_should_halt(&h, now, Duration::from_secs(60), 5));
    }

    #[test]
    fn loop_guard_does_not_halt_below_threshold() {
        let mut h = RestartHistory::default();
        let now = fixed_now();
        for offset in [0, 5, 10] {
            h.record(now - chrono::Duration::seconds(offset));
        }
        assert!(!loop_guard_should_halt(&h, now, Duration::from_secs(60), 5));
    }

    #[test]
    fn loop_guard_does_not_halt_when_restarts_outside_window() {
        let mut h = RestartHistory::default();
        let now = fixed_now();
        for offset in [120, 140, 160, 180, 200] {
            h.record(now - chrono::Duration::seconds(offset));
        }
        assert!(!loop_guard_should_halt(&h, now, Duration::from_secs(60), 5));
    }

    #[test]
    fn save_and_load_restart_history_round_trips_via_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("restart-state.json");
        let mut h = RestartHistory::default();
        h.record(fixed_now());
        save_restart_history(&path, &h).unwrap();
        let loaded = load_restart_history(&path);
        assert_eq!(loaded.recent_restarts_at, h.recent_restarts_at);
    }

    #[test]
    fn load_restart_history_missing_file_returns_default() {
        let path = std::env::temp_dir().join(format!(
            "bpmux-self-update-test-missing-{}.json",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let h = load_restart_history(&path);
        assert_eq!(h.recent_restarts_at.len(), 0);
    }

    #[test]
    fn load_restart_history_corrupt_file_returns_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("corrupt.json");
        fs::write(&path, b"not valid json").unwrap();
        let h = load_restart_history(&path);
        assert_eq!(h.recent_restarts_at.len(), 0);
    }

    #[tokio::test]
    async fn binary_watcher_fires_signal_when_file_mtime_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fake-binary");
        fs::write(&path, b"v1").unwrap();
        let ct = CancellationToken::new();
        let signal = RestartSignal::new();
        let handle = spawn_binary_watcher_at(
            path.clone(),
            ct.clone(),
            signal.clone(),
            Duration::from_millis(50),
        );
        tokio::time::sleep(Duration::from_millis(120)).await;
        tokio::time::sleep(Duration::from_millis(1100)).await;
        fs::write(&path, b"v2-changed").unwrap();
        for _ in 0..50 {
            if signal.was_fired() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            signal.was_fired(),
            "watcher must fire signal on mtime change"
        );
        assert!(
            ct.is_cancelled(),
            "watcher must cancel ct to trigger graceful shutdown"
        );
        let _ = handle.await;
    }

    #[test]
    fn resolve_watch_path_honors_env_override() {
        let tmp = tempfile::tempdir().unwrap();
        let custom = tmp.path().join("installed-bpmux");
        fs::write(&custom, b"v1").unwrap();
        let resolved =
            resolve_watch_path_with_override(Some(custom.display().to_string())).unwrap();
        assert_eq!(resolved, custom);
    }

    #[test]
    fn resolve_watch_path_falls_back_to_current_exe_when_no_override() {
        let resolved = resolve_watch_path_with_override(None).unwrap();
        assert_eq!(resolved, std::env::current_exe().unwrap());
    }

    #[tokio::test]
    async fn binary_watcher_returns_early_when_file_missing() {
        let path = std::env::temp_dir().join(format!(
            "nonexistent-binary-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let ct = CancellationToken::new();
        let signal = RestartSignal::new();
        let handle = spawn_binary_watcher_at(path, ct, signal.clone(), Duration::from_millis(50));
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(!signal.was_fired(), "missing-file watcher must not fire");
        let _ = handle.await;
    }

    #[tokio::test]
    async fn binary_watcher_exits_on_cancellation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fake-binary");
        fs::write(&path, b"v1").unwrap();
        let ct = CancellationToken::new();
        let signal = RestartSignal::new();
        let handle =
            spawn_binary_watcher_at(path, ct.clone(), signal.clone(), Duration::from_secs(60));
        tokio::time::sleep(Duration::from_millis(50)).await;
        ct.cancel();
        tokio::time::timeout(Duration::from_millis(500), handle)
            .await
            .expect("watcher must exit promptly on ct.cancel()")
            .ok();
        assert!(
            !signal.was_fired(),
            "ct-cancel exit path must not spuriously fire signal"
        );
    }
}
