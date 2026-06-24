use crate::product_management::story::StoryState;

pub fn sort_board_ready_by_membership(stories: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let current = stories.iter().find_map(|s| {
        if s.get("is_current").and_then(|v| v.as_bool()) == Some(true) {
            s.get("number").and_then(|v| v.as_u64())
        } else {
            None
        }
    });
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
                let number = s.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
                match membership_position_for_current(s, current) {
                    Some(pos) => (0u8, pos, number),
                    None => (1u8, u32::MAX, number),
                }
            });
        }
    }
    by_state.into_iter().flat_map(|(_, list)| list).collect()
}

fn membership_position_for_current(s: &serde_json::Value, current: Option<u64>) -> Option<u32> {
    let current = current?;
    let membership = s.get("iteration_membership")?;
    if membership.is_null() {
        return None;
    }
    if membership.get("iteration_number")?.as_u64()? != current {
        return None;
    }
    membership.get("position")?.as_u64().map(|p| p as u32)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "ready_order_label_tests.rs"]
mod tests;
