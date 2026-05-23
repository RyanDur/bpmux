use crate::board::domain::{Story, StoryState};
use crate::board::label_codec::LabelCodec;

pub fn parse_ready_order_label<C: LabelCodec>(codec: &C, label: &str) -> Option<u32> {
    codec.decode_ready_order(label)
}

pub fn is_ready_order_label<C: LabelCodec>(codec: &C, label: &str) -> bool {
    codec.is_ready_order_label(label)
}

pub fn story_ready_order_position<C: LabelCodec>(codec: &C, story: &Story) -> Option<u32> {
    story
        .raw_labels
        .iter()
        .filter_map(|l| codec.decode_ready_order(l))
        .min()
}

pub fn stories_with_ready_order<C: LabelCodec>(codec: &C, stories: &[Story]) -> Vec<u64> {
    let mut ordered: Vec<(u64, u32)> = stories
        .iter()
        .filter(|s| !s.is_closed && s.state == StoryState::Ready)
        .filter_map(|s| story_ready_order_position(codec, s).map(|pos| (s.number, pos)))
        .collect();
    ordered.sort_by_key(|(_, pos)| *pos);
    ordered.into_iter().map(|(n, _)| n).collect()
}

pub fn sort_board_by_ready_order_labels<C: LabelCodec>(
    codec: &C,
    stories: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let mut by_state: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
    for s in stories {
        let state = s
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        match by_state.iter_mut().find(|(k, _)| k == &state) {
            Some((_, list)) => list.push(s),
            None => by_state.push((state, vec![s])),
        }
    }
    for (state, list) in by_state.iter_mut() {
        if StoryState::parse(state) == Some(StoryState::Ready) {
            list.sort_by_cached_key(|s| {
                let label_pos = ready_order_from_value(codec, s).map(u64::from);
                let number = s.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
                (label_pos.unwrap_or(u64::MAX), number)
            });
        }
    }
    by_state.into_iter().flat_map(|(_, list)| list).collect()
}

fn ready_order_from_value<C: LabelCodec>(codec: &C, s: &serde_json::Value) -> Option<u32> {
    s.get("labels")?.as_array()?.iter().find_map(|l| {
        let name = l.as_str().or_else(|| l.get("name")?.as_str())?;
        codec.decode_ready_order(name)
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "ready_order_label_tests.rs"]
mod tests;
