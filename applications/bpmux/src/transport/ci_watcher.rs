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
//! Poll cadence is 60s — push-on-transition still keeps the glyph
//! tight via WS broadcast; the poll itself only matters as a safety
//! net for reconnect windows + secondary-visibility (the frontend's
//! own 15s poll covers per-client refresh). 60 polls/hr leaves
//! ample headroom under GitHub's 5000/hr authenticated rate limit
//! AND avoids triggering secondary rate-limit (burst-pattern
//! detection) which the prior 10s cadence kept self-perpetuating —
//! once tripped, every 10s tick re-tripped it indefinitely.
//! 2026-05-02 retro: cross-role convergence on cadence-reduction
//! after empirically observing the self-perpetuating-loop class
//! across yesterday + today's debugging arcs. Structural close via
//! #1140 (push-substrate via webhook) when shipped; this is the
//! immediate mitigation.
//!
//! Pure classification via `classify_transition` lets unit tests
//! cover the "emit exactly when conclusion changes, not on every
//! tick" contract without mocking the tracker.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::board::adapters::github::GitHubSourceAdapter;
use crate::orchestrator::{CiAction, CiSnapshot, Orchestrator};
use crate::session;

pub const POLL_INTERVAL: Duration = Duration::from_secs(10);
pub const MAX_BACKOFF: Duration = Duration::from_secs(3600);
pub const WATCHER_UNHEALTHY_THRESHOLD: Duration = Duration::from_secs(120);

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
pub fn spawn(
    orchestrator: Arc<Orchestrator>,
    tracker: Arc<GitHubSourceAdapter>,
    ct: CancellationToken,
) {
    let observer = orchestrator.rate_limit_observer().clone();
    tokio::spawn(async move {
        let mut last_etag: Option<String> = None;
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        let base_backoff = Duration::from_secs(60);
        let max_backoff = MAX_BACKOFF;
        let mut backoff = base_backoff;
        loop {
            tokio::select! {
                _ = ct.cancelled() => break,
                _ = interval.tick() => {}
            }
            let conditional = tracker
                .list_latest_run_on_main_conditional(last_etag.as_deref())
                .await;
            let report = match conditional {
                Ok(crate::bridge::tracker::ConditionalRunEntry::NotModified) => {
                    backoff = base_backoff;
                    continue;
                }
                Ok(crate::bridge::tracker::ConditionalRunEntry::Modified { entry, etag }) => {
                    backoff = base_backoff;
                    last_etag = etag;
                    entry
                }
                Ok(crate::bridge::tracker::ConditionalRunEntry::Empty) => {
                    backoff = base_backoff;
                    continue;
                }
                Err(e) => {
                    if let Some(decision) = crate::github::classify_and_decide(
                        &e,
                        observer.snapshot(),
                        backoff,
                        std::time::SystemTime::now(),
                    ) {
                        session::diag::log_diag(&format!(
                            "ci_watcher: rate-limited; backoff signal={} wait={}s",
                            crate::github::signal_label(decision.signal),
                            decision.wait.as_secs()
                        ));
                        tokio::select! {
                            _ = ct.cancelled() => break,
                            _ = tokio::time::sleep(decision.wait) => {}
                        }
                        backoff = (backoff * 2).min(max_backoff);
                    } else {
                        session::diag::log_diag(&format!("ci_watcher: fetch failed: {e}"));
                    }
                    continue;
                }
            };
            let run_id = report.database_id;
            let runs = vec![crate::ci::RunEntry {
                status: report.status,
                conclusion: report.conclusion,
                head_sha: report.head_sha,
                display_title: report.display_title,
                database_id: report.database_id,
                annotation_message: report.annotation_message,
            }];
            let classified = crate::ci::classify_runs(&runs);
            let current: &'static str = match classified.state {
                crate::ci::CiState::Green => "success",
                crate::ci::CiState::Red => "failure",
                crate::ci::CiState::Running => "in_progress",
                crate::ci::CiState::QuotaExhausted => "quota_exhausted",
                crate::ci::CiState::Unknown => "unknown",
            };
            let snapshot = CiSnapshot {
                state: classified.state,
                run_id,
                head_sha: runs[0].head_sha.clone(),
                display_title: runs[0].display_title.clone(),
                conclusion: current,
            };
            orchestrator.dispatch(CiAction::CiUpdated(snapshot)).await;
        }
    });
}
