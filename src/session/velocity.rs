//! Velocity metrics computed from workflow-signal timestamps (#209a).
//!
//! The team is velocity-blind without an objective source of truth —
//! "are we shipping?" gets answered by feel, which drifts into
//! co-rationalization (see team lesson 2026-04-15). This module parses
//! the bpmux workflow signals that already fire on every story
//! lifecycle transition (#197: start/deliver/accept/fail-desk-check)
//! and derives cycle-time and rework statistics from their timestamps.
//!
//! Pure computation over a slice of `Message`. No IO, no external
//! state, no self-report — every number traces to an observable
//! signal.
//!
//! Paired with the MCP `view_velocity` tool in `bridge/tools.rs`.
//! TUI rendering ships in #210.

use crate::session::message::{Message, MessageType};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::Serialize;
use std::collections::HashMap;

/// Fewer than this many delivered stories in the window means the
/// averages aren't load-bearing — `sparse` is true and callers should
/// treat the medians as indicative only. See AC: "if the data is
/// sparse, the view says so honestly."
pub const SPARSE_THRESHOLD: usize = 3;

/// Rolling window over which metrics are computed.
pub const WINDOW_DAYS: i64 = 14;

/// Signal parsers are anchored on the `#N ` prefix and a lifecycle
/// verb so they're robust to the surrounding copy. These match what
/// `bridge/tools_stories.rs` emits via `build_*_signal`.
fn parse_lifecycle(text: &str) -> Option<(u64, Lifecycle)> {
    let hash_idx = text.find('#')?;
    let rest = &text[hash_idx + 1..];
    let (num_str, after) = rest
        .find(|c: char| !c.is_ascii_digit())
        .map(|i| rest.split_at(i))?;
    let number: u64 = num_str.parse().ok()?;
    let after = after.trim_start();
    if after.starts_with("started") {
        Some((number, Lifecycle::Started))
    } else if after.contains("delivered") {
        Some((number, Lifecycle::Delivered))
    } else if after.starts_with("accepted") {
        Some((number, Lifecycle::Accepted))
    } else if after.starts_with("failed desk-check") {
        Some((number, Lifecycle::FailedDeskCheck))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Lifecycle {
    Started,
    Delivered,
    Accepted,
    FailedDeskCheck,
}

/// Story type per the 3-type vocabulary adopted 2026-04-16 (#264).
/// Maps 1:1 to Pivotal Tracker's convention and GitHub's native
/// issue types. The old 4-label invention (vertical / stranded-
/// vertical / honest-plumbing / half-slice) is retired; stranded-
/// vertical and half-slice live on only as desk-check rejection
/// reasons on a Feature, not as distinct types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryType {
    /// User-facing business value. Counts for velocity; owner-accepted.
    Feature,
    /// Defect fix. Zero velocity; accepter depends on surface
    /// (user-visible → Ryan; internal-only → PM).
    Bug,
    /// Tech debt, refactor, infrastructure. Zero velocity; team-accepted.
    Chore,
}

/// Classify a story by its GitHub labels first, falling back to the
/// title prefix if no type label is present. Defaults to Feature for
/// untagged stories so they don't silently drop from velocity —
/// "insufficient data" is honest but we shouldn't under-count by
/// default. Supports legacy title prefixes during the migration
/// window (honest-plumbing / vertical / stranded-vertical / half-
/// slice); new filings use the canonical [Feature]/[Bug]/[Chore].
pub fn classify_story(labels: &[String], title: &str) -> StoryType {
    for l in labels {
        // Label matching is case-insensitive so "Bug" / "bug" /
        // "BUG" all classify the same — GitHub ships with lowercase
        // "bug" by default and we don't want that mismatching our
        // canonical "Bug" casing.
        match l.to_ascii_lowercase().as_str() {
            "feature" => return StoryType::Feature,
            "bug" => return StoryType::Bug,
            "chore" => return StoryType::Chore,
            _ => {}
        }
    }
    let lower = title.to_lowercase();
    // Prefix matching strips the closing bracket so compound tags
    // (e.g., `[honest-plumbing/security]`) still classify — the
    // first recognized word wins. Ordered most-specific first.
    if lower.starts_with("[feature")
        || lower.starts_with("[vertical")
        || lower.starts_with("[stranded-vertical")
    {
        return StoryType::Feature;
    }
    if lower.starts_with("[bug") || lower.starts_with("bug:") {
        return StoryType::Bug;
    }
    if lower.starts_with("[chore")
        || lower.starts_with("[honest-plumbing")
        || lower.starts_with("[half-slice")
    {
        return StoryType::Chore;
    }
    StoryType::Feature
}

/// Return true iff the story counts toward business-value velocity.
/// Under the 3-type vocabulary that's exactly Feature. Bug and
/// Chore are zero-velocity by Pivotal convention.
pub fn is_feature_verdict(labels: &[String], title: &str) -> bool {
    matches!(classify_story(labels, title), StoryType::Feature)
}

#[derive(Serialize, Debug, PartialEq)]
pub struct VelocityMetrics {
    pub window_days: i64,
    pub delivered_in_window: usize,
    pub delivered_per_day: f64,
    pub median_pickup_to_deliver_hours: Option<f64>,
    pub median_deliver_to_accept_hours: Option<f64>,
    pub rework_rate: Option<f64>,
    pub sparse: bool,
    pub note: Option<String>,
}

/// Compute velocity metrics from the session's message log.
///
/// `is_feature(issue_number)` returns true iff the issue counts
/// toward business-value velocity. Honest-plumbing stories (tech
/// debt) return false — their lifecycle events still fire but
/// don't move the feature-velocity needle, matching Pivotal's
/// chores-outside-velocity convention and Ryan's 2026-04-16
/// direction ("the only thing that counts as a story point is
/// business value"). Callers with no tracker / label access can
/// pass `|_| true` to disable the filter and count everything.
///
/// Filters to the last `WINDOW_DAYS` (bounded by `now`) and pairs
/// lifecycle events by issue number to derive durations.
pub fn compute(
    messages: &[Message],
    now: DateTime<Utc>,
    is_feature: impl Fn(u64) -> bool,
) -> VelocityMetrics {
    let window_start = now - ChronoDuration::days(WINDOW_DAYS);

    // Collect events keyed by (issue_number, lifecycle) → timestamps.
    // Multiple instances per issue are possible (e.g. delivered twice
    // after a fail-desk-check) — we keep them all.
    // Stories that don't count as business-value are filtered at
    // ingestion so downstream calculations see only feature events.
    let mut events: HashMap<(u64, Lifecycle), Vec<DateTime<Utc>>> = HashMap::new();
    for m in messages {
        if m.message_type != MessageType::Chat {
            continue;
        }
        if m.ts < window_start || m.ts > now {
            continue;
        }
        if let Some((number, lc)) = parse_lifecycle(&m.text) {
            if !is_feature(number) {
                continue;
            }
            events.entry((number, lc)).or_default().push(m.ts);
        }
    }

    let delivered_events: Vec<DateTime<Utc>> = events
        .iter()
        .filter(|((_, lc), _)| *lc == Lifecycle::Delivered)
        .flat_map(|(_, ts)| ts.clone())
        .collect();
    let delivered_in_window = delivered_events.len();

    let delivered_per_day = delivered_in_window as f64 / WINDOW_DAYS as f64;

    // Median pickup→deliver: for each delivered event, find the most
    // recent Started event for the same issue that preceded it.
    let mut pickup_durations: Vec<f64> = Vec::new();
    for ((number, lc), deliver_times) in &events {
        if *lc != Lifecycle::Delivered {
            continue;
        }
        let starts = events.get(&(*number, Lifecycle::Started));
        for d in deliver_times {
            if let Some(starts) = starts
                && let Some(best_start) = starts.iter().filter(|s| s <= &d).max()
            {
                let hours = (*d - *best_start).num_minutes() as f64 / 60.0;
                if hours >= 0.0 {
                    pickup_durations.push(hours);
                }
            }
        }
    }
    let median_pickup_to_deliver_hours = median(&mut pickup_durations);

    // Median deliver→accept: for each Accepted event, find the most
    // recent Delivered preceding it for the same issue.
    let mut accept_durations: Vec<f64> = Vec::new();
    for ((number, lc), accept_times) in &events {
        if *lc != Lifecycle::Accepted {
            continue;
        }
        let delivers = events.get(&(*number, Lifecycle::Delivered));
        for a in accept_times {
            if let Some(delivers) = delivers
                && let Some(best_deliver) = delivers.iter().filter(|d| d <= &a).max()
            {
                let hours = (*a - *best_deliver).num_minutes() as f64 / 60.0;
                if hours >= 0.0 {
                    accept_durations.push(hours);
                }
            }
        }
    }
    let median_deliver_to_accept_hours = median(&mut accept_durations);

    // Rework rate: count of fail-desk-check events / count of
    // delivered events in the window. Unlike simple rework-per-story,
    // this measures "how often a delivery comes back for fixes" — the
    // designer-desk-check hit rate.
    let fail_count: usize = events
        .iter()
        .filter(|((_, lc), _)| *lc == Lifecycle::FailedDeskCheck)
        .map(|(_, ts)| ts.len())
        .sum();
    let rework_rate = if delivered_in_window == 0 {
        None
    } else {
        Some(fail_count as f64 / delivered_in_window as f64)
    };

    let sparse = delivered_in_window < SPARSE_THRESHOLD;
    let note = if sparse {
        Some(format!(
            "sparse data: only {delivered_in_window} delivered \
             stor{} in last {WINDOW_DAYS} days (threshold {SPARSE_THRESHOLD}); \
             medians and rate are indicative only.",
            if delivered_in_window == 1 { "y" } else { "ies" },
        ))
    } else {
        None
    };

    VelocityMetrics {
        window_days: WINDOW_DAYS,
        delivered_in_window,
        delivered_per_day,
        median_pickup_to_deliver_hours,
        median_deliver_to_accept_hours,
        rework_rate,
        sparse,
        note,
    }
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    Some(if n.is_multiple_of(2) {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    } else {
        values[n / 2]
    })
}
