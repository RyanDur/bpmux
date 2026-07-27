pub struct KeyedDiff {
    pub upserts: Vec<serde_json::Value>,
    pub removed: Vec<u64>,
    pub order: Option<Vec<u64>>,
}

fn item_id(item: &serde_json::Value) -> Option<u64> {
    item.get("id").and_then(|v| v.as_u64())
}

pub fn diff_keyed(previous: &[serde_json::Value], current: &[serde_json::Value]) -> KeyedDiff {
    let prev_by_id: Vec<(u64, &serde_json::Value)> = previous
        .iter()
        .filter_map(|item| item_id(item).map(|id| (id, item)))
        .collect();
    let current_ids: Vec<u64> = current.iter().filter_map(item_id).collect();
    let previous_ids: Vec<u64> = prev_by_id.iter().map(|(id, _)| *id).collect();

    let upserts = current
        .iter()
        .filter(|item| {
            item_id(item).is_some_and(|id| {
                !prev_by_id
                    .iter()
                    .any(|(prev_id, prev)| *prev_id == id && *prev == *item)
            })
        })
        .cloned()
        .collect();
    let removed = prev_by_id
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| !current_ids.contains(id))
        .collect();
    let order = (previous_ids != current_ids).then_some(current_ids);
    KeyedDiff {
        upserts,
        removed,
        order,
    }
}
