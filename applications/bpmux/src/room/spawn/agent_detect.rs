use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;

use super::Osc7CwdParser;
use crate::location::register_dir_user;
use crate::room::LivePtyRegistry;
use crate::store::{AppAction, Dispatch, Presence, RosterAction, Select};

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

pub fn spawn_agent_run_detector(
    terminal_id: String,
    mut output: broadcast::Receiver<Vec<u8>>,
    registry: Arc<LivePtyRegistry>,
    dispatch: Dispatch,
    select: Select,
    hub: Arc<crate::hub::Hub>,
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
                        && !select.select(|state| {
                            crate::store::routing::is_terminal_linked(&state.routing, &terminal_id)
                        })
                        && let Ok(dir_user) =
                            register_dir_user(std::path::Path::new(&mint.cwd))
                    {
                        let actions = select.select(|state| {
                            crate::store::routing::register_agent_actions(
                                &state.routing,
                                &dir_user.id,
                                &dir_user.name,
                                &terminal_id,
                                &mint.job_start_ts,
                            )
                        });
                        for action in actions {
                            dispatch.dispatch(action.into());
                        }
                        crate::room::drain_missed(
                            &hub,
                            &registry,
                            &select,
                            &dispatch,
                            &dir_user.id,
                            &terminal_id,
                            &dir_user.name,
                            &mint.job_start_ts,
                        );
                        dispatch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
                            routing_id: dir_user.id.clone(),
                            identity: dir_user.name.clone(),
                            presence: Presence::Online,
                        }));
                    }
                    if foreground {
                        present = present
                            || select.select(|state| {
                                crate::store::routing::is_terminal_linked(&state.routing, &terminal_id)
                            });
                    } else if present {
                        leave_terminal(&dispatch, &select, &terminal_id);
                        present = false;
                        detector.on_left();
                    }
                }
            }
        }
        leave_terminal(&dispatch, &select, &terminal_id);
    });
}

fn leave_terminal(dispatch: &Dispatch, select: &Select, terminal_id: &str) {
    if let Some(routing_id) =
        select.select(|state| crate::store::routing::user_for_terminal(&state.routing, terminal_id))
    {
        dispatch.dispatch(AppAction::Roster(RosterAction::RosterMemberRemoved {
            routing_id,
        }));
    }
}
