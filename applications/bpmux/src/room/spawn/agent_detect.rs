use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;

use super::Osc7CwdParser;
use crate::room::LivePtyRegistry;
use crate::session::read_or_create_dir_session;
use crate::session::routing_identity::RoutingStore;

const AGENT_RUN_GATE: Duration = Duration::from_millis(1000);
const DETECTOR_POLL: Duration = Duration::from_millis(200);

pub struct MintRequest {
    pub cwd: String,
    pub job_start_ts: String,
}

pub struct AgentRunDetector {
    parser: Osc7CwdParser,
    cwd: Option<String>,
    job_start: Option<(Instant, String)>,
    registered: bool,
    threshold: Duration,
}

impl AgentRunDetector {
    pub fn new(threshold: Duration) -> Self {
        Self {
            parser: Osc7CwdParser::new(),
            cwd: None,
            job_start: None,
            registered: false,
            threshold,
        }
    }

    pub fn feed_output(&mut self, bytes: &[u8]) {
        if let Some(path) = self.parser.feed(bytes) {
            self.cwd = Some(path);
        }
    }

    pub fn observe(&mut self, foreground: bool, now: Instant, now_ts: &str) -> Option<MintRequest> {
        if !foreground {
            self.job_start = None;
            return None;
        }
        if self.job_start.is_none() {
            self.job_start = Some((now, now_ts.to_string()));
        }
        if self.registered {
            return None;
        }
        let (start_instant, start_ts) = match &self.job_start {
            Some((instant, ts)) => (*instant, ts.clone()),
            None => return None,
        };
        if now.duration_since(start_instant) >= self.threshold {
            let cwd = self.cwd.clone()?;
            self.registered = true;
            return Some(MintRequest {
                cwd,
                job_start_ts: start_ts,
            });
        }
        None
    }
}

pub fn spawn_agent_run_detector(
    terminal_id: String,
    mut output: broadcast::Receiver<Vec<u8>>,
    registry: Arc<LivePtyRegistry>,
    routing: Arc<RoutingStore>,
    session: Arc<crate::session::Session>,
    orchestrator: Option<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
) {
    tokio::spawn(async move {
        let mut detector = AgentRunDetector::new(AGENT_RUN_GATE);
        let mut poll = tokio::time::interval(DETECTOR_POLL);
        loop {
            tokio::select! {
                chunk = output.recv() => {
                    match chunk {
                        Ok(bytes) if bytes.is_empty() => break,
                        Ok(bytes) => detector.feed_output(&bytes),
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = poll.tick() => {
                    let foreground = match registry.has_foreground_job(&terminal_id) {
                        Some(foreground) => foreground,
                        None => break,
                    };
                    let now_ts = chrono::Utc::now().to_rfc3339();
                    if let Some(mint) = detector.observe(foreground, Instant::now(), &now_ts)
                        && !routing.is_terminal_linked(&terminal_id)
                        && let Ok(dir_session) =
                            read_or_create_dir_session(std::path::Path::new(&mint.cwd))
                    {
                        routing.register_agent(
                            &dir_session.id,
                            &dir_session.name,
                            &terminal_id,
                            &mint.job_start_ts,
                        );
                        let display = routing
                            .display_for(&dir_session.id)
                            .unwrap_or_else(|| dir_session.name.clone());
                        if let Some(orchestrator) = &orchestrator {
                            crate::transport::ws::dispatch_roster_join(
                                orchestrator,
                                &dir_session.id,
                                &display,
                            )
                            .await;
                        }
                        crate::room::drain_missed(
                            &session,
                            &registry,
                            &routing,
                            &dir_session.id,
                            &terminal_id,
                            &dir_session.name,
                            &mint.job_start_ts,
                        );
                    }
                }
            }
        }
        if let Some(session_id) = routing.session_for_terminal(&terminal_id)
            && let Some(orchestrator) = &orchestrator
        {
            crate::transport::ws::dispatch_roster_leave(orchestrator, &session_id).await;
        }
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn empty_message_session() -> Arc<crate::session::Session> {
        let dir = std::env::temp_dir().join(format!("bpmux-detect-{}", uuid::Uuid::new_v4()));
        let (tx, _rx) = tokio::sync::broadcast::channel(16);
        Arc::new(crate::session::Session::new(dir, "ryan".to_string(), tx))
    }

    fn osc7(path: &str) -> Vec<u8> {
        format!("\x1b]7;file://host{path}\x07").into_bytes()
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
        detector.feed_output(&osc7("/Users/ryan/project"));
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
            mint.cwd, "/Users/ryan/project",
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

    #[tokio::test]
    async fn a_sustained_foreground_job_in_a_cwd_auto_registers_the_agent_headless() {
        use crate::room::{LivePtyRegistry, osc7_shell_integration, spawn_pty_retained};
        use crate::session::routing_identity::RoutingStore;

        let store_dir = tempfile::TempDir::new().unwrap();
        let routing = Arc::new(RoutingStore::load(store_dir.path().to_path_buf()));
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
            routing.clone(),
            empty_message_session(),
            None,
        );

        registry.write_to(&terminal_id, format!("cd {work_path}\n").as_bytes());
        registry.write_to(&terminal_id, b"sleep 4\n");

        let marker = work.path().join(".bpmux-session-id");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            if marker.is_file() && !routing.resolve(&work_name).is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        assert!(
            marker.is_file(),
            "the agent-run detector read_or_create'd the work-dir's .bpmux-session-id marker on a SUSTAINED foreground-job (cd-emit gave the cwd, the sleep is the foreground-child)"
        );
        let session = crate::session::read_or_create_dir_session(work.path()).unwrap();
        assert_eq!(
            routing.resolve(&work_name),
            vec![session.id],
            "a sustained foreground-job in the work-dir auto-registered the agent (display defaults to the folder basename, no spawn-time role) HEADLESS — no viewer subscribed to the pane; the running IS the agnostic signal (falsifier-1)"
        );
    }

    #[tokio::test]
    async fn a_cd_through_a_dir_without_running_an_agent_mints_no_id() {
        use crate::room::{LivePtyRegistry, osc7_shell_integration, spawn_pty_retained};
        use crate::session::routing_identity::RoutingStore;

        let store_dir = tempfile::TempDir::new().unwrap();
        let routing = Arc::new(RoutingStore::load(store_dir.path().to_path_buf()));
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
            routing.clone(),
            empty_message_session(),
            None,
        );

        registry.write_to(&terminal_id, format!("cd {passed_path}\n").as_bytes());
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

        let marker = passed_through.path().join(".bpmux-session-id");
        assert!(
            !marker.is_file(),
            "a bare cd-through (no foreground-child job) mints NO id in the passed-through dir — falsifier-3 litter-free by construction (cd is a shell builtin, no child → has_foreground_job stays false)"
        );
        assert!(
            routing.resolve(&passed_name).is_empty(),
            "nothing registers without an agent actually running there — the passed-through dir's basename routes to nobody"
        );
    }

    #[tokio::test]
    async fn an_agent_started_in_place_with_no_cd_registers_via_the_startup_emit() {
        use crate::room::{LivePtyRegistry, osc7_shell_integration, spawn_pty_retained};
        use crate::session::routing_identity::RoutingStore;

        let store_dir = tempfile::TempDir::new().unwrap();
        let routing = Arc::new(RoutingStore::load(store_dir.path().to_path_buf()));
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
        let mut live =
            spawn_pty_retained(&shell, &integration.args, Some(work.path()), &env).unwrap();
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
            routing.clone(),
            empty_message_session(),
            None,
        );

        registry.write_to(&terminal_id, b"sleep 4\n");

        let marker = work.path().join(".bpmux-session-id");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            if marker.is_file() && !routing.resolve(&work_name).is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(
            marker.is_file(),
            "an agent that STARTS in the work-dir (no later cd) registers via the STARTUP OSC-7 emit (zsh .zshrc-load / bash PROMPT_COMMAND) — the cwd is tracked without a cd"
        );
        let session = crate::session::read_or_create_dir_session(work.path()).unwrap();
        assert_eq!(
            routing.resolve(&work_name),
            vec![session.id],
            "start-in-place + a sustained job registers off the startup-emit (display = folder basename)"
        );
    }
}
