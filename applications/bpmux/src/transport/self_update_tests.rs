#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::self_update::resolve_watch_path_with_override;
use super::self_update::*;
use chrono::TimeZone;
use std::fs;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

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
    let resolved = resolve_watch_path_with_override(Some(custom.display().to_string())).unwrap();
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
    let handle = spawn_binary_watcher_at(path, ct.clone(), signal.clone(), Duration::from_secs(60));
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
