use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::Serialize;
use std::collections::HashMap;

pub const SPARSE_THRESHOLD: usize = 3;

pub const WINDOW_DAYS: i64 = 14;

#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub story_id: u64,
    pub new_state: String,
    pub at: DateTime<Utc>,
}

fn lifecycle_events(transitions: &[Transition]) -> Vec<(DateTime<Utc>, u64, Lifecycle)> {
    let mut ordered: Vec<&Transition> = transitions.iter().collect();
    ordered.sort_by_key(|t| t.at);
    let mut last_state: HashMap<u64, &str> = HashMap::new();
    let mut events = Vec::new();
    for t in ordered {
        match t.new_state.as_str() {
            "in-progress" => {
                if last_state.get(&t.story_id).copied() == Some("acceptance") {
                    events.push((t.at, t.story_id, Lifecycle::FailedDeskCheck));
                }
                events.push((t.at, t.story_id, Lifecycle::Started));
            }
            "acceptance" => events.push((t.at, t.story_id, Lifecycle::Delivered)),
            "accepted" => events.push((t.at, t.story_id, Lifecycle::Accepted)),
            _ => {}
        }
        last_state.insert(t.story_id, t.new_state.as_str());
    }
    events
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Lifecycle {
    Started,
    Delivered,
    Accepted,
    FailedDeskCheck,
}

mod wip_history;
pub use wip_history::WipPoint;

#[cfg(test)]
mod tests;

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
    pub wip_history: Vec<WipPoint>,
}

pub fn compute(
    transitions: &[Transition],
    now: DateTime<Utc>,
    is_feature: impl Fn(u64) -> bool,
) -> VelocityMetrics {
    let window_start = now - ChronoDuration::days(WINDOW_DAYS);

    let mut events: HashMap<(u64, Lifecycle), Vec<DateTime<Utc>>> = HashMap::new();
    for (at, number, lc) in lifecycle_events(transitions) {
        if at < window_start || at > now {
            continue;
        }
        if !is_feature(number) {
            continue;
        }
        events.entry((number, lc)).or_default().push(at);
    }

    let unique_delivered: std::collections::HashSet<u64> = events
        .iter()
        .filter(|((_, lc), _)| *lc == Lifecycle::Delivered)
        .map(|((number, _), _)| *number)
        .collect();
    let delivered_in_window = unique_delivered.len();

    let delivered_per_day = delivered_in_window as f64 / WINDOW_DAYS as f64;

    let mut pickup_durations = stage_durations(&events, Lifecycle::Started, Lifecycle::Delivered);
    let median_pickup_to_deliver_hours = median(&mut pickup_durations);

    let mut accept_durations = stage_durations(&events, Lifecycle::Delivered, Lifecycle::Accepted);
    let median_deliver_to_accept_hours = median(&mut accept_durations);

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
        wip_history: wip_history::derive_wip_history(transitions, now),
    }
}

fn stage_durations(
    events: &HashMap<(u64, Lifecycle), Vec<DateTime<Utc>>>,
    from: Lifecycle,
    to: Lifecycle,
) -> Vec<f64> {
    let mut durations = Vec::new();
    for ((number, lifecycle), end_times) in events {
        if *lifecycle != to {
            continue;
        }
        let start_times = events.get(&(*number, from));
        for end in end_times {
            if let Some(hours) = stage_hours(start_times, end) {
                durations.push(hours);
            }
        }
    }
    durations
}

fn stage_hours(start_times: Option<&Vec<DateTime<Utc>>>, end: &DateTime<Utc>) -> Option<f64> {
    let best_start = start_times?.iter().filter(|start| start <= &end).max()?;
    let hours = (*end - *best_start).num_minutes() as f64 / 60.0;
    (hours >= 0.0).then_some(hours)
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
