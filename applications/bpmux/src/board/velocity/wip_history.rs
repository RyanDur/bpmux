use std::collections::HashSet;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::Serialize;

use super::{Lifecycle, WINDOW_DAYS, parse_lifecycle};
use crate::message::{Message, MessageType};

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct WipPoint {
    pub date: String,
    pub wip_count: usize,
}

pub(super) fn derive_wip_history(messages: &[Message], now: DateTime<Utc>) -> Vec<WipPoint> {
    let window_start = now - ChronoDuration::days(WINDOW_DAYS);
    let mut events: Vec<(DateTime<Utc>, u64, Lifecycle)> = Vec::new();
    for m in messages {
        if m.message_type != MessageType::Chat {
            continue;
        }
        if m.ts < window_start || m.ts > now {
            continue;
        }
        if let Some((number, lc)) = parse_lifecycle(&m.text)
            && matches!(lc, Lifecycle::Started | Lifecycle::Delivered)
        {
            events.push((m.ts, number, lc));
        }
    }
    events.sort_by_key(|(ts, _, _)| *ts);

    let mut in_progress: HashSet<u64> = HashSet::new();
    let mut points: Vec<WipPoint> = Vec::new();
    for (ts, number, lc) in events {
        match lc {
            Lifecycle::Started => {
                in_progress.insert(number);
            }
            Lifecycle::Delivered => {
                in_progress.remove(&number);
            }
            _ => {}
        }
        let date = ts.format("%Y-%m-%d").to_string();
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
