use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;

use chrono::{DateTime, Utc};

use crate::board::history::{PlacementHistory, PlacementRecord};
use crate::product_management::story::{StoryId, Work};
use workflow::StageId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Position {
    Placement(StageId),
    Coarse(Work),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReworkBreakdown {
    pub total: usize,
    pub per_story: Vec<(StoryId, usize)>,
}

fn position_of(record: &PlacementRecord) -> Position {
    match &record.placement {
        Some(stage) => Position::Placement(stage.clone()),
        None => Position::Coarse(record.to.clone()),
    }
}

fn stories_in_ts_order(history: &PlacementHistory) -> Vec<Vec<&PlacementRecord>> {
    let mut order: Vec<StoryId> = Vec::new();
    let mut by_story: HashMap<StoryId, Vec<&PlacementRecord>> = HashMap::new();
    for record in history.records() {
        if !by_story.contains_key(&record.story) {
            order.push(record.story);
        }
        by_story.entry(record.story).or_default().push(record);
    }
    order
        .into_iter()
        .map(|story| {
            let mut records = by_story.remove(&story).unwrap_or_default();
            records.sort_by_key(|record| record.ts);
            records
        })
        .collect()
}

pub fn sub_section_timings(history: &PlacementHistory) -> Vec<(Position, f64)> {
    let mut order: Vec<Position> = Vec::new();
    let mut spans: HashMap<Position, Vec<f64>> = HashMap::new();
    for records in stories_in_ts_order(history) {
        for pair in records.windows(2) {
            let position = position_of(pair[0]);
            if !spans.contains_key(&position) {
                order.push(position.clone());
            }
            spans
                .entry(position)
                .or_default()
                .push(hours_between(pair[0].ts, pair[1].ts));
        }
    }
    order
        .into_iter()
        .filter_map(|position| {
            let mut hours = spans.remove(&position)?;
            median(&mut hours).map(|value| (position, value))
        })
        .collect()
}

pub fn desk_check_rework(history: &PlacementHistory) -> ReworkBreakdown {
    let mut per_story: Vec<(StoryId, usize)> = Vec::new();
    let mut total = 0usize;
    for records in stories_in_ts_order(history) {
        let count = reentries(&records);
        if count > 0 {
            let story = records[0].story;
            per_story.push((story, count));
            total += count;
        }
    }
    ReworkBreakdown { total, per_story }
}

fn reentries(records: &[&PlacementRecord]) -> usize {
    let mut departed: HashSet<Position> = HashSet::new();
    let mut previous: Option<Position> = None;
    let mut count = 0usize;
    for record in records {
        let current = position_of(record);
        match previous {
            Some(prior) if prior != current => {
                if departed.contains(&current) {
                    count += 1;
                }
                departed.insert(prior);
            }
            _ => {}
        }
        previous = Some(current);
    }
    count
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
