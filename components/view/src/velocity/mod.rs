use chrono::{DateTime, Duration as ChronoDuration, Utc};
use message::{Message, MessageType};
use serde::Serialize;
use std::collections::HashMap;

pub const SPARSE_THRESHOLD: usize = 3;

pub const WINDOW_DAYS: i64 = 14;

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
    messages: &[Message],
    now: DateTime<Utc>,
    is_feature: impl Fn(u64) -> bool,
) -> VelocityMetrics {
    let window_start = now - ChronoDuration::days(WINDOW_DAYS);

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

    let unique_delivered: std::collections::HashSet<u64> = events
        .iter()
        .filter(|((_, lc), _)| *lc == Lifecycle::Delivered)
        .map(|((number, _), _)| *number)
        .collect();
    let delivered_in_window = unique_delivered.len();

    let delivered_per_day = delivered_in_window as f64 / WINDOW_DAYS as f64;

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
        wip_history: wip_history::derive_wip_history(messages, now),
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
