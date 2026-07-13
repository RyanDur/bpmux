use std::cmp::Ordering;
use std::collections::HashMap;
use std::num::NonZeroUsize;

use chrono::{DateTime, Utc};

use crate::history::SharedHistory;
use crate::iteration::{Iteration, IterationId};
use crate::story::{StoryId, Verdict};

const READY: &str = "ready";
const ACCEPTANCE: &str = "acceptance";

fn is_succeed(verdict: &Verdict, step: &str) -> bool {
    matches!(verdict, Verdict::Succeed { step: s } if s.as_str() == step)
}

fn is_fail(verdict: &Verdict, step: &str) -> bool {
    matches!(verdict, Verdict::Fail { step: s } if s.as_str() == step)
}

pub fn accepted(history: &SharedHistory, story: StoryId) -> bool {
    history
        .records()
        .iter()
        .any(|record| record.story == story && is_succeed(&record.verdict, ACCEPTANCE))
}

pub fn velocity_series(
    history: &SharedHistory,
    iterations: &[Iteration],
    is_feature: impl Fn(StoryId) -> bool,
) -> Vec<(IterationId, usize)> {
    let mut complete: Vec<&Iteration> = iterations.iter().filter(|it| it.complete).collect();
    complete.sort_by_key(|it| it.iteration_order);
    complete
        .into_iter()
        .map(|it| {
            let count = it
                .stories
                .iter()
                .filter(|story| is_feature(**story) && accepted(history, **story))
                .count();
            (it.id, count)
        })
        .collect()
}

pub fn velocity(
    history: &SharedHistory,
    iterations: &[Iteration],
    is_feature: impl Fn(StoryId) -> bool,
    window: NonZeroUsize,
) -> Option<f64> {
    let series = velocity_series(history, iterations, is_feature);
    if series.is_empty() {
        return None;
    }
    let tail = &series[series.len().saturating_sub(window.get())..];
    let sum: usize = tail.iter().map(|(_, count)| count).sum();
    Some(sum as f64 / tail.len() as f64)
}

pub fn lead_time_median_hours(history: &SharedHistory) -> Option<f64> {
    let mut first_ready: HashMap<StoryId, DateTime<Utc>> = HashMap::new();
    let mut acceptance: HashMap<StoryId, DateTime<Utc>> = HashMap::new();
    for record in history.records() {
        if is_succeed(&record.verdict, READY) {
            first_ready
                .entry(record.story)
                .and_modify(|ts| *ts = (*ts).min(record.ts))
                .or_insert(record.ts);
        } else if is_succeed(&record.verdict, ACCEPTANCE) {
            acceptance
                .entry(record.story)
                .and_modify(|ts| *ts = (*ts).max(record.ts))
                .or_insert(record.ts);
        }
    }
    let mut leads: Vec<f64> = acceptance
        .into_iter()
        .filter_map(|(story, accept_ts)| {
            first_ready
                .get(&story)
                .map(|ready_ts| hours_between(*ready_ts, accept_ts))
        })
        .collect();
    median(&mut leads)
}

pub fn acceptance_rejection_rate(history: &SharedHistory) -> Option<f64> {
    let mut fails = 0usize;
    let mut ceremonies = 0usize;
    for record in history.records() {
        if is_fail(&record.verdict, ACCEPTANCE) {
            fails += 1;
            ceremonies += 1;
        } else if is_succeed(&record.verdict, ACCEPTANCE) {
            ceremonies += 1;
        }
    }
    if ceremonies == 0 {
        return None;
    }
    Some(fails as f64 / ceremonies as f64)
}

fn hours_between(start: DateTime<Utc>, end: DateTime<Utc>) -> f64 {
    (end - start).num_seconds() as f64 / 3600.0
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some((values[mid - 1] + values[mid]) / 2.0)
    }
}
