#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::agent_detect::{AgentRunDetector, spawn_agent_run_detector};
use crate::location::test_support::DataBaseGuard;
use crate::store::SeededStore;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn empty_message_hub() -> Arc<crate::hub::Hub> {
    let dir = std::env::temp_dir().join(format!("bpmux-detect-{}", uuid::Uuid::new_v4()));
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    Arc::new(crate::hub::Hub::new(dir, "owner".to_string(), tx))
}

fn osc7(path: &str) -> Vec<u8> {
    format!("\x1b]7;file://host{path}\x07").into_bytes()
}

fn resolve(store: &SeededStore, display: &str) -> Vec<String> {
    store
        .select()
        .select(|s| crate::store::routing::resolve(&s.routing, display))
}

const GATE: Duration = Duration::from_millis(1000);

#[test]
fn a_bare_cd_with_no_foreground_job_never_mints() {
    let mut detector = AgentRunDetector::new(GATE);
    detector.feed_output(&osc7("/work"));
    let now = Instant::now();
    assert!(
        detector
            .observe(false, now, "2026-06-12T10:00:00Z")
            .is_none(),
        "a bare cd (no foreground-child job) never mints — falsifier-3 litter-free by construction"
    );
}

#[test]
fn a_sub_threshold_foreground_job_does_not_mint() {
    let mut detector = AgentRunDetector::new(GATE);
    detector.feed_output(&osc7("/work"));
    let start = Instant::now();
    assert!(
        detector.observe(true, start, "ts").is_none(),
        "a just-started job is below the duration-gate — no mint yet"
    );
    assert!(
        detector
            .observe(true, start + Duration::from_millis(500), "ts")
            .is_none(),
        "still under the ~1s gate — a transient command must not mint"
    );
}

#[test]
fn a_transient_job_that_ends_before_the_gate_resets_and_never_mints() {
    let mut detector = AgentRunDetector::new(GATE);
    detector.feed_output(&osc7("/work"));
    let start = Instant::now();
    detector.observe(true, start, "ts");
    assert!(
        detector
            .observe(false, start + Duration::from_millis(300), "ts")
            .is_none(),
        "the transient command ended (foreground gone) — the gate resets"
    );
    assert!(
        detector
            .observe(false, start + Duration::from_millis(2000), "ts")
            .is_none(),
        "a sub-second transient (ls/cat) that ended never mints — its job-start was reset on job-end"
    );
}

#[test]
fn a_sustained_job_in_a_known_cwd_mints_at_the_gate_with_the_job_start_ts() {
    let mut detector = AgentRunDetector::new(GATE);
    detector.feed_output(&osc7("/Users/owner/project"));
    let start = Instant::now();
    assert!(
        detector
            .observe(true, start, "2026-06-12T10:00:00Z")
            .is_none(),
        "below the gate yet"
    );
    let mint = detector
        .observe(true, start + GATE, "2026-06-12T10:00:01Z")
        .expect("a SUSTAINED foreground-job (>= gate) in a known cwd mints");
    assert_eq!(
        mint.cwd, "/Users/owner/project",
        "mints in the cwd the agent works in (the OSC-7-tracked dir)"
    );
    assert_eq!(
        mint.job_start_ts, "2026-06-12T10:00:00Z",
        "the cursor-floor uses the JOB-START ts (not the mint-instant) — so a message sent during the gate-window drains (falsifier-4)"
    );
}

#[test]
fn a_sustained_job_with_no_cwd_yet_waits_then_mints_once_the_cwd_arrives() {
    let mut detector = AgentRunDetector::new(GATE);
    let start = Instant::now();
    detector.observe(true, start, "ts");
    assert!(
        detector.observe(true, start + GATE, "ts").is_none(),
        "no OSC-7 cwd seen yet → cannot mint; must wait for the cwd emission, never guess"
    );
    detector.feed_output(&osc7("/late/cwd"));
    let mint = detector
        .observe(true, start + Duration::from_millis(1200), "ts")
        .expect("mints once the cwd is known");
    assert_eq!(mint.cwd, "/late/cwd");
}

#[test]
fn an_already_registered_terminal_does_not_re_mint() {
    let mut detector = AgentRunDetector::new(GATE);
    detector.feed_output(&osc7("/work"));
    let start = Instant::now();
    detector.observe(true, start, "ts");
    assert!(
        detector.observe(true, start + GATE, "ts").is_some(),
        "the first sustained job mints"
    );
    detector.feed_output(&osc7("/elsewhere"));
    assert!(
        detector
            .observe(true, start + Duration::from_millis(3000), "ts2")
            .is_none(),
        "an already-registered terminal does not re-mint — one running agent, one registration (a later cd within the session does not re-register)"
    );
}

#[test]
fn after_the_agent_leaves_a_fresh_foreground_job_re_mints() {
    let mut detector = AgentRunDetector::new(GATE);
    detector.feed_output(&osc7("/work"));
    let start = Instant::now();
    detector.observe(true, start, "ts");
    assert!(
        detector.observe(true, start + GATE, "ts").is_some(),
        "the first sustained job mints"
    );
    detector.on_left();
    let restart = start + Duration::from_millis(5000);
    detector.observe(true, restart, "ts2");
    assert!(
        detector.observe(true, restart + GATE, "ts2").is_some(),
        "once the agent left (foreground returned to the shell), on_left clears the registration so a fresh sustained foreground job — a restart in the same terminal — re-mints and re-joins"
    );
}

#[tokio::test]
async fn a_sustained_foreground_job_in_a_cwd_auto_registers_the_agent_headless() {
    use crate::room::{LivePtyRegistry, osc7_shell_integration, spawn_pty_retained};

    let data_base = tempfile::TempDir::new().unwrap();
    let _base_guard = DataBaseGuard::set(data_base.path());

    let store = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    let registry = Arc::new(LivePtyRegistry::new());

    let work = tempfile::TempDir::new().unwrap();
    let work_path = work.path().to_string_lossy().to_string();
    let work_name = work
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let integration = osc7_shell_integration(&shell).unwrap();
    let mut env: Vec<(&str, &str)> = vec![("TERM", "xterm-256color")];
    for (key, value) in &integration.env {
        env.push((key.as_str(), value.as_str()));
    }
    let mut live = spawn_pty_retained(&shell, &integration.args, None, &env).unwrap();
    if let Some(dir) = integration.zdotdir {
        live.attach_tempdir(dir);
    }
    let output = live.subscribe();
    let terminal_id = "term-agnostic".to_string();
    registry.insert(terminal_id.clone(), live);

    spawn_agent_run_detector(
        terminal_id.clone(),
        output,
        registry.clone(),
        store.dispatcher(),
        store.select(),
        empty_message_hub(),
    );

    registry.write_to(&terminal_id, format!("cd {work_path}\n").as_bytes());
    registry.write_to(&terminal_id, b"sleep 4\n");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while std::time::Instant::now() < deadline {
        if !resolve(&store, &work_name).is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    assert!(
        !work.path().join(".bpmux-user-id").exists(),
        "auto-registration writes the identity into the data-home registry, NOT a marker in the detected cwd — the agent's workspace stays clean"
    );
    let user = crate::location::register_dir_user(work.path()).unwrap();
    assert_eq!(
        resolve(&store, &work_name),
        vec![user.id],
        "a sustained foreground-job in the work-dir auto-registered the agent (display defaults to the folder basename, no spawn-time role) HEADLESS — no viewer subscribed to the pane; the running IS the agnostic signal (falsifier-1)"
    );
}

#[tokio::test]
async fn a_cd_through_a_dir_without_running_an_agent_mints_no_id() {
    use crate::room::{LivePtyRegistry, osc7_shell_integration, spawn_pty_retained};

    let data_base = tempfile::TempDir::new().unwrap();
    let _base_guard = DataBaseGuard::set(data_base.path());

    let store = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    let registry = Arc::new(LivePtyRegistry::new());
    let passed_through = tempfile::TempDir::new().unwrap();
    let passed_path = passed_through.path().to_string_lossy().to_string();
    let passed_name = passed_through
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let integration = osc7_shell_integration(&shell).unwrap();
    let mut env: Vec<(&str, &str)> = vec![("TERM", "xterm-256color")];
    for (key, value) in &integration.env {
        env.push((key.as_str(), value.as_str()));
    }
    let mut live = spawn_pty_retained(&shell, &integration.args, None, &env).unwrap();
    if let Some(dir) = integration.zdotdir {
        live.attach_tempdir(dir);
    }
    let output = live.subscribe();
    let terminal_id = "term-passthrough".to_string();
    registry.insert(terminal_id.clone(), live);
    spawn_agent_run_detector(
        terminal_id.clone(),
        output,
        registry.clone(),
        store.dispatcher(),
        store.select(),
        empty_message_hub(),
    );

    registry.write_to(&terminal_id, format!("cd {passed_path}\n").as_bytes());
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

    assert!(
        !data_base.path().join("users").join("by-dir").exists(),
        "a bare cd-through (no foreground-child job) mints NO id for the passed-through dir — falsifier-3 litter-free by construction (cd is a shell builtin, no child → has_foreground_job stays false); were this to regress, the sandboxed base catches the mint instead of the real data home"
    );
    assert!(
        resolve(&store, &passed_name).is_empty(),
        "nothing registers without an agent actually running there — the passed-through dir's basename routes to nobody"
    );
}

#[tokio::test]
async fn an_agent_started_in_place_with_no_cd_registers_via_the_startup_emit() {
    use crate::room::{LivePtyRegistry, osc7_shell_integration, spawn_pty_retained};

    let data_base = tempfile::TempDir::new().unwrap();
    let _base_guard = DataBaseGuard::set(data_base.path());

    let store = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    let registry = Arc::new(LivePtyRegistry::new());
    let work = tempfile::TempDir::new().unwrap();
    let work_name = work
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let integration = osc7_shell_integration(&shell).unwrap();
    let mut env: Vec<(&str, &str)> = vec![("TERM", "xterm-256color")];
    for (key, value) in &integration.env {
        env.push((key.as_str(), value.as_str()));
    }
    let mut live = spawn_pty_retained(&shell, &integration.args, Some(work.path()), &env).unwrap();
    if let Some(dir) = integration.zdotdir {
        live.attach_tempdir(dir);
    }
    let output = live.subscribe();
    let terminal_id = "term-inplace".to_string();
    registry.insert(terminal_id.clone(), live);
    spawn_agent_run_detector(
        terminal_id.clone(),
        output,
        registry.clone(),
        store.dispatcher(),
        store.select(),
        empty_message_hub(),
    );

    registry.write_to(&terminal_id, b"sleep 4\n");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while std::time::Instant::now() < deadline {
        if !resolve(&store, &work_name).is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        !work.path().join(".bpmux-user-id").exists(),
        "start-in-place registration also leaves the workspace clean — identity lives in the data-home registry, not a cwd marker"
    );
    let user = crate::location::register_dir_user(work.path()).unwrap();
    assert_eq!(
        resolve(&store, &work_name),
        vec![user.id],
        "start-in-place + a sustained job registers off the startup-emit (display = folder basename)"
    );
}
