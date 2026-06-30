use crate::product_management::story::StoryState;

pub fn sort_board_ready_by_membership(
    stories: Vec<serde_json::Value>,
    current_order: &[u64],
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
                let number = s.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                match current_order.iter().position(|&id| id == number) {
                    Some(idx) => (0u8, idx, number),
                    None => (1u8, usize::MAX, number),
                }
            });
        }
    }
    by_state.into_iter().flat_map(|(_, list)| list).collect()
}
