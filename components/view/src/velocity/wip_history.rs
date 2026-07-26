use std::collections::HashSet;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::Serialize;

use super::{Lifecycle, Transition, WINDOW_DAYS, lifecycle_events};

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct WipPoint {
    pub date: String,
    pub wip_count: usize,
}

pub(super) fn derive_wip_history(transitions: &[Transition], now: DateTime<Utc>) -> Vec<WipPoint> {
    let window_start = now - ChronoDuration::days(WINDOW_DAYS);
    let mut events: Vec<(DateTime<Utc>, u64, Lifecycle)> = lifecycle_events(transitions)
        .into_iter()
        .filter(|(at, _, lc)| {
            *at >= window_start
                && *at <= now
                && matches!(lc, Lifecycle::Started | Lifecycle::Delivered)
        })
        .collect();
    events.sort_by_key(|(at, _, _)| *at);

    let mut in_progress: HashSet<u64> = HashSet::new();
    let mut points: Vec<WipPoint> = Vec::new();
    for (at, number, lc) in events {
        match lc {
            Lifecycle::Started => {
                in_progress.insert(number);
            }
            Lifecycle::Delivered => {
                in_progress.remove(&number);
            }
            _ => {}
        }
        let date = at.format("%Y-%m-%d").to_string();
        if points.last().map(|p| p.date.as_str()) == Some(date.as_str()) {
            if let Some(last) = points.last_mut() {
                last.wip_count = in_progress.len();
            }
        } else {
            points.push(WipPoint {
                date,
                wip_count: in_progress.len(),
            });
        }
    }
    points
}
