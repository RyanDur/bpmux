use message::StateDeltaPayload;

fn story_id(story: &serde_json::Value) -> Option<u64> {
    story.get("id").and_then(|v| v.as_u64())
}

pub fn diff_story_frames(
    previous: &[serde_json::Value],
    current: &[serde_json::Value],
) -> Vec<StateDeltaPayload> {
    let prev_by_id: Vec<(u64, &serde_json::Value)> = previous
        .iter()
        .filter_map(|s| story_id(s).map(|id| (id, s)))
        .collect();
    let current_ids: Vec<u64> = current.iter().filter_map(story_id).collect();
    let previous_ids: Vec<u64> = prev_by_id.iter().map(|(id, _)| *id).collect();

    let mut frames: Vec<StateDeltaPayload> = Vec::new();
    for story in current {
        let Some(id) = story_id(story) else { continue };
        let unchanged = prev_by_id
            .iter()
            .any(|(prev_id, prev)| *prev_id == id && *prev == story);
        if !unchanged {
            frames.push(StateDeltaPayload::StoryUpserted {
                story: story.clone(),
            });
        }
    }
    for (id, _) in &prev_by_id {
        if !current_ids.contains(id) {
            frames.push(StateDeltaPayload::StoryRemoved { id: *id });
        }
    }
    if previous_ids != current_ids {
        frames.push(StateDeltaPayload::StoryOrder { ids: current_ids });
    }
    frames
}
