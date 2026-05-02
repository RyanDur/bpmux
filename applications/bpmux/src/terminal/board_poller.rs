//! Background board-state poller (#219).
//!
//! Separates data flow from views. A tokio task polls GitHub's REST
//! API every `POLL_INTERVAL` and writes results into a shared
//! `BoardSnapshot`. The view reads whatever's current — no fetch on
//! open, no loading state.
//!
//! The fetcher uses octocrab (typed REST client) instead of shelling
//! out to gh. Auth via `GH_TOKEN` env var, matching gh's convention.
//!
//! Error surfaces are honest: rate-limit, auth-missing, network-down
//! each map to a specific message. The board view renders the
//! last-successful data plus the error, not a blank screen.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use super::board::BoardStory;

/// Poll cadence. 60s = ~120 calls/hour, well under GitHub's 5000/hr
/// authenticated limit.
pub const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Snapshot of the board data the poller writes and the view reads.
/// Cloneable so the view can grab a cheap consistent view of the
/// state without holding the lock during rendering.
#[derive(Clone, Default, Debug)]
pub struct BoardSnapshot {
    pub stories: Vec<BoardStory>,
    pub last_updated: Option<DateTime<Utc>>,
    pub last_error: Option<BoardFetchError>,
}

/// Error taxonomy for board fetches. Re-exported here so existing
/// `terminal::board_poller::BoardFetchError` imports keep working;
/// the type itself moved to `crate::github` so both `terminal` and
/// `bridge` depend on a shared below-them module instead of sibling
/// imports.
pub use crate::github::BoardFetchError;

/// Commands sent to the poller to trigger off-cycle behavior.
/// Currently just "refetch now" — used by signal-driven refresh.
#[derive(Debug)]
pub enum PollerCommand {
    /// Force an immediate refetch. Sent when a workflow signal
    /// (accept_story, deliver_story, etc.) indicates state changed.
    RefreshNow,
}

/// Format "last updated X ago" for the view. Graceful degradation
/// across seconds/minutes/hours — never shows "0m ago."
pub fn format_last_updated(last: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let delta = now.signed_duration_since(last);
    let secs = delta.num_seconds();
    if secs < 60 {
        format!("last updated {secs}s ago")
    } else if secs < 3600 {
        let m = delta.num_minutes();
        format!("last updated {m}m ago")
    } else {
        let h = delta.num_hours();
        format!("last updated {h}h ago")
    }
}

/// Parse octocrab's Issue type into a BoardStory. Kept as a pure
/// function so the mapping is testable without a live API.
pub fn issue_to_story(issue: &octocrab::models::issues::Issue) -> BoardStory {
    use super::board::StoryState;

    let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
    let has = |name: &str| labels.iter().any(|l| l == name);

    let is_closed = matches!(issue.state, octocrab::models::IssueState::Closed);
    let state = if is_closed {
        StoryState::Accepted
    } else if has("deferred") {
        StoryState::Deferred
    } else if has("delivered") {
        StoryState::Delivered
    } else if has("started") || has("finished") {
        StoryState::InProgress
    } else if has("ready") {
        StoryState::Ready
    } else {
        StoryState::Icebox
    };

    let reason = if state == StoryState::Deferred {
        labels
            .iter()
            .find(|l| l.starts_with("deferred:"))
            .map(|l| l.strip_prefix("deferred:").unwrap_or(l).trim().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let created = issue.created_at.format("%Y-%m-%d").to_string();

    BoardStory {
        number: issue.number,
        title: issue.title.clone(),
        state,
        body: issue.body.clone().unwrap_or_default(),
        assignees: issue.assignees.iter().map(|a| a.login.clone()).collect(),
        created,
        reason,
        labels,
    }
}

pub use crate::github::classify_error;

/// Spawn the background poller. Returns a sender the caller can use
/// to trigger off-cycle refetches (signal-driven refresh). The task
/// exits cleanly when the sender is dropped.
pub fn spawn(
    snapshot: Arc<Mutex<BoardSnapshot>>,
    owner: String,
    repo: String,
) -> mpsc::Sender<PollerCommand> {
    let (tx, mut rx) = mpsc::channel::<PollerCommand>(16);
    tokio::spawn(async move {
        let client = match build_client() {
            Ok(c) => c,
            Err(e) => {
                write_error(&snapshot, e);
                return;
            }
        };
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    fetch_once(&client, &owner, &repo, &snapshot).await;
                }
                cmd = rx.recv() => {
                    match cmd {
                        Some(PollerCommand::RefreshNow) => {
                            fetch_once(&client, &owner, &repo, &snapshot).await;
                            interval.reset();
                        }
                        None => break, // all senders dropped
                    }
                }
            }
        }
    });
    tx
}

fn build_client() -> Result<octocrab::Octocrab, BoardFetchError> {
    if let Ok(base_url) = std::env::var("BPMUX_GH_BASE_URL")
        && !base_url.is_empty()
    {
        return octocrab::OctocrabBuilder::new()
            .base_uri(base_url)
            .map_err(|e| classify_error(&e))?
            .build()
            .map_err(|e| classify_error(&e));
    }
    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| BoardFetchError::AuthInvalid)?;
    octocrab::OctocrabBuilder::new()
        .personal_token(token)
        .build()
        .map_err(|e| classify_error(&e))
}

async fn fetch_once(
    client: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    snapshot: &Arc<Mutex<BoardSnapshot>>,
) {
    let page = client
        .issues(owner, repo)
        .list()
        .state(octocrab::params::State::All)
        .per_page(100)
        .send()
        .await;
    match page {
        Ok(page) => {
            let stories: Vec<BoardStory> = page.items.iter().map(issue_to_story).collect();
            if let Ok(mut snap) = snapshot.lock() {
                snap.stories = stories;
                snap.last_updated = Some(Utc::now());
                snap.last_error = None;
            }
        }
        Err(e) => {
            write_error(snapshot, classify_error(&e));
        }
    }
}

fn write_error(snapshot: &Arc<Mutex<BoardSnapshot>>, err: BoardFetchError) {
    if let Ok(mut snap) = snapshot.lock() {
        snap.last_error = Some(err);
    }
}
