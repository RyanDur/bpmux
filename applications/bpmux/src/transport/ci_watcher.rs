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
use crate::session;
use crate::transport::ci_state_store::CiStateStore;

pub const POLL_INTERVAL: Duration = Duration::from_secs(60);

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
    session: Arc<session::Session>,
    tracker: Arc<GitHubSourceAdapter>,
    ci_state_store: CiStateStore,
    ct: CancellationToken,
) {
    let observer = tracker.rate_limit_observer();
    tokio::spawn(async move {
        let mut last_seen: Option<Observation> = None;
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        let base_backoff = Duration::from_secs(60);
        let max_backoff = Duration::from_secs(3600);
        let mut backoff = base_backoff;
        let mut first = true;
        loop {
            tokio::select! {
                _ = ct.cancelled() => break,
                _ = interval.tick() => {}
            }
            let report = match tracker.list_latest_run_on_main().await {
                Ok(Some(run)) => {
                    backoff = base_backoff;
                    run
                }
                Ok(None) => {
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
            // Reuse the existing classifier so watcher + REST endpoint
            // + MCP tool all agree on what "success / failure /
            // in_progress / unknown" looks like. Single source of
            // truth for the state vocabulary.
            let run_id = report.database_id;
            let runs = vec![crate::ci::RunEntry {
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
            let classified = crate::ci::classify_runs(&runs);
            let current: &'static str = match classified.state {
                crate::ci::CiState::Green => "success",
                crate::ci::CiState::Red => "failure",
                crate::ci::CiState::Running => "in_progress",
                crate::ci::CiState::QuotaExhausted => "quota_exhausted",
                crate::ci::CiState::Unknown => "unknown",
            };
            let cached_report = crate::bridge::tools::build_ci_state_report(&runs);
            ci_state_store.write(cached_report);
            let observation: Observation = (run_id, current);
            if first {
                last_seen = Some(observation);
                first = false;
                continue;
            }
            match classify_transition(last_seen, observation) {
                Transition::Changed { .. } => {
                    last_seen = Some(observation);
                    let event = crate::ci::CiEvent {
                        state: classified.state,
                        sha: runs[0].head_sha.clone(),
                        title: runs[0].display_title.clone(),
                        run_id,
                    };
                    broadcast(&session, &event);
                }
                Transition::Same | Transition::NoData => {}
            }
        }
    });
}

/// Per-completion broadcast (#1140 rework):
/// - Fires the existing `CI_STATE_UPDATE_MARKER` (System-class, filtered by
///   the bridge before reaching agents) so the renderer's CiGlyph
///   re-fetches `/api/ci`.
/// - ALSO fires the agent-visible Activity broadcast + dev-addressed
///   workflow signal via `terminal::ci`'s pure builders. Same shape as
///   the TUI poller emits, so an agent receives one signal regardless
///   of which poller observes the completion. Closes #1140's
///   silent-hold-loop class at the SERVER's runtime path (this watcher
///   is what `server.rs:163` spawns; the TUI's `terminal::ci::spawn_poller`
///   is a separate poller that doesn't run in Electron-spawned bpmux).
fn broadcast(session: &session::Session, event: &crate::ci::CiEvent) {
    let marker_msg = session::Message {
        from: "bpmux".to_string(),
        text: crate::bridge::CI_STATE_UPDATE_MARKER.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::System,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&marker_msg) {
        session::diag::log_diag(&format!("ci_watcher: marker broadcast failed: {e}"));
    }
    let Some(text) = crate::ci::compute_broadcast_text(event) else {
        return;
    };
    let multi_depth = crate::ci::build_multi_depth_for_transition(event);
    let activity_msg = session::Message {
        from: "bpmux".to_string(),
        text: text.clone(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::Activity,
        multi_depth: Some(multi_depth),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&activity_msg) {
        session::diag::log_diag(&format!("ci_watcher: activity broadcast failed: {e}"));
    }
    let dev_signal = crate::ci::build_dev_ci_signal(&text);
    if let Err(e) = session.broadcast(&dev_signal) {
        session::diag::log_diag(&format!("ci_watcher: dev signal failed: {e}"));
    }
}
