//! Backend CI state watcher (#372). Polls GitHub at a fixed cadence,
//! tracks the last-seen `(run_id, conclusion)` pair, and broadcasts
//! `CI_STATE_UPDATE_MARKER` when either component differs — i.e. a
//! new run on main OR a conclusion flip on the same run. The
//! renderer's CiGlyph subscribes to that marker and refetches
//! `/api/ci` immediately, turning CI state from pull-only (the 15s
//! frontend poll from #361) into push-driven — state changes land
//! within WS latency instead of waiting up to 15s for the next
//! client tick. The 15s poll stays as a safety net so a disconnect-
//! then-reconnect window doesn't strand the glyph on stale state.
//!
//! #395: keyed on `(run_id, conclusion)` not conclusion alone. The
//! prior version swallowed broadcasts when CI stayed red across
//! successive pushes (billing failures, flaky substrate) — only the
//! first red fired, later reds fell through `Same`. Now every new
//! run_id fires regardless of whether its conclusion matches the
//! prior run's.
//!
//! Poll cadence is 10s — faster than the frontend's 15s so a
//! transition is more likely to be caught server-side (where every
//! connected renderer benefits from the broadcast) than per-client
//! poll-side. 360 polls/hr well under GitHub's 5000/hr authenticated
//! rate limit.
//!
//! Pure classification via `classify_transition` lets unit tests
//! cover the "emit exactly when conclusion changes, not on every
//! tick" contract without mocking the tracker.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::bridge::tracker::StoryTracker;
use crate::session;

pub const POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Classification of a tick result vs prior state. `Changed` triggers
/// the broadcast; `Same` is a no-op. `NoData` (empty runs list) leaves
/// prior state untouched so a transient empty response doesn't burn
/// the next genuine transition.
///
/// #395: the key used for dedup is the `(run_id, conclusion)` tuple.
/// A new run_id with the same conclusion as the prior run is still a
/// `Changed` — the team needs visibility on each run's outcome, not
/// just transitions between distinct conclusions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    Changed { new_conclusion: &'static str },
    Same,
    NoData,
}

/// Observation key: the pair we dedup on. `run_id` is optional because
/// GitHub's response schema allows it to be missing; in practice it's
/// always populated for real runs, but the type stays faithful. When
/// `run_id` is `None`, dedup falls back to conclusion-only semantics
/// for that slot (same as pre-#395 behavior).
pub type Observation = (Option<u64>, &'static str);

pub fn classify_transition(prev: Option<Observation>, current: Observation) -> Transition {
    match prev {
        Some(p) if p == current => Transition::Same,
        _ => Transition::Changed {
            new_conclusion: current.1,
        },
    }
}

/// Spawn the background watcher. Returns immediately; the task runs
/// until the cancellation token fires. Silent on tracker / API
/// errors beyond a diag-log entry — the frontend's 15s poll catches
/// up next tick regardless.
pub fn spawn<T: StoryTracker>(
    session: Arc<session::Session>,
    tracker: Arc<T>,
    ct: CancellationToken,
) {
    tokio::spawn(async move {
        let mut last_seen: Option<Observation> = None;
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        // First tick fires immediately — seeds last_seen with the
        // current observation without broadcasting. Subsequent ticks
        // are the real transition detectors.
        let mut first = true;
        loop {
            tokio::select! {
                _ = ct.cancelled() => break,
                _ = interval.tick() => {}
            }
            let report = match tracker.list_latest_run_on_main().await {
                Ok(Some(run)) => run,
                Ok(None) => {
                    // Repo has no runs yet — nothing to watch.
                    continue;
                }
                Err(e) => {
                    session::diag::log_diag(&format!("ci_watcher: fetch failed: {e}"));
                    continue;
                }
            };
            // Reuse the existing classifier so watcher + REST endpoint
            // + MCP tool all agree on what "success / failure /
            // in_progress / unknown" looks like. Single source of
            // truth for the state vocabulary.
            let run_id = report.database_id;
            let runs = vec![crate::terminal::ci::RunEntry {
                status: report.status,
                conclusion: report.conclusion,
                head_sha: report.head_sha,
                display_title: report.display_title,
                database_id: report.database_id,
                // #397: annotation_message flows from tracker's
                // list_latest_run_on_main when conclusion=failure,
                // enabling QuotaExhausted classification.
                annotation_message: report.annotation_message,
            }];
            let classified = crate::terminal::ci::classify_runs(&runs);
            let current: &'static str = match classified.state {
                crate::terminal::ci::CiState::Green => "success",
                crate::terminal::ci::CiState::Red => "failure",
                crate::terminal::ci::CiState::Running => "in_progress",
                crate::terminal::ci::CiState::QuotaExhausted => "quota_exhausted",
                crate::terminal::ci::CiState::Unknown => "unknown",
            };
            let observation: Observation = (run_id, current);
            if first {
                last_seen = Some(observation);
                first = false;
                continue;
            }
            match classify_transition(last_seen, observation) {
                Transition::Changed { .. } => {
                    last_seen = Some(observation);
                    broadcast(&session);
                }
                Transition::Same | Transition::NoData => {}
            }
        }
    });
}

fn broadcast(session: &session::Session) {
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: crate::bridge::CI_STATE_UPDATE_MARKER.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::System,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("ci_watcher: broadcast failed: {e}"));
    }
}
