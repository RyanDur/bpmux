use crate::session::Message;
use chrono::{DateTime, Utc};
use serde::Serialize;

const CONTEXT_WINDOW: usize = 5;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub messages: Vec<Message>,
    pub match_index: usize,
}

pub struct SearchFilters {
    pub query: String,
    pub sender: Option<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub channel: Option<String>,
}

pub fn search(messages: &[Message], filters: &SearchFilters) -> Vec<SearchResult> {
    let query_lower = filters.query.to_lowercase();
    let mut results = Vec::new();
    let mut used: Vec<bool> = vec![false; messages.len()];

    for (i, msg) in messages.iter().enumerate() {
        if used[i] || !msg.is_chat() {
            continue;
        }
        if !matches_filters(msg, filters, &query_lower) {
            continue;
        }
        let start = i.saturating_sub(CONTEXT_WINDOW);
        let end = (i + CONTEXT_WINDOW + 1).min(messages.len());
        let cluster: Vec<Message> = messages[start..end]
            .iter()
            .filter(|m| m.is_chat())
            .cloned()
            .collect();
        used[start..end].fill(true);
        results.push(SearchResult {
            match_index: i - start,
            messages: cluster,
        });
    }
    results
}

fn matches_filters(msg: &Message, filters: &SearchFilters, query_lower: &str) -> bool {
    if !query_lower.is_empty() && !msg.text.to_lowercase().contains(query_lower) {
        return false;
    }
    if let Some(sender) = &filters.sender
        && msg.from.split(':').next().unwrap_or(&msg.from) != sender
    {
        return false;
    }
    if let Some(after) = filters.after
        && msg.ts < after
    {
        return false;
    }
    if let Some(before) = filters.before
        && msg.ts > before
    {
        return false;
    }
    true
}
