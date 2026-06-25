use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;

use super::Osc7CwdParser;
use crate::location::read_or_create_dir_session;
use crate::room::LivePtyRegistry;
use crate::routing::RoutingStore;

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

    pub fn on_left(&mut self) {
        self.registered = false;
        self.job_start = None;
    }
}

type DetectorOrchestrator = Arc<crate::store::Store>;

pub fn spawn_agent_run_detector(
    terminal_id: String,
    mut output: broadcast::Receiver<Vec<u8>>,
    registry: Arc<LivePtyRegistry>,
    routing: Arc<RoutingStore>,
    session: Arc<crate::session::Session>,
    orchestrator: Option<DetectorOrchestrator>,
) {
    tokio::spawn(async move {
        let mut detector = AgentRunDetector::new(AGENT_RUN_GATE);
        let mut poll = tokio::time::interval(DETECTOR_POLL);
        let mut present = false;
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
                        crate::room::drain_missed(
                            &session,
                            &registry,
                            &routing,
                            &dir_session.id,
                            &terminal_id,
                            &dir_session.name,
                            &mint.job_start_ts,
                        );
                        if let Some(orchestrator) = &orchestrator {
                            crate::transport::ws::dispatch_roster_join(
                                orchestrator,
                                &dir_session.id,
                                &dir_session.name,
                            )
                            .await;
                        }
                    }
                    if foreground {
                        present = present || routing.is_terminal_linked(&terminal_id);
                    } else if present {
                        leave_terminal(&orchestrator, &routing, &terminal_id).await;
                        present = false;
                        detector.on_left();
                    }
                }
            }
        }
        leave_terminal(&orchestrator, &routing, &terminal_id).await;
    });
}

async fn leave_terminal(
    orchestrator: &Option<DetectorOrchestrator>,
    routing: &RoutingStore,
    terminal_id: &str,
) {
    if let (Some(orchestrator), Some(routing_id)) =
        (orchestrator, routing.session_for_terminal(terminal_id))
    {
        crate::transport::ws::dispatch_roster_leave(orchestrator, &routing_id).await;
    }
}
