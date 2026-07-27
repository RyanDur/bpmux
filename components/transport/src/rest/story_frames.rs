use message::StateDeltaPayload;

use super::keyed_frames::{KeyedDiff, diff_keyed};

pub fn map_frames(
    diff: KeyedDiff,
    upsert: impl Fn(serde_json::Value) -> StateDeltaPayload,
    removed: impl Fn(u64) -> StateDeltaPayload,
    order: impl Fn(Vec<u64>) -> StateDeltaPayload,
) -> Vec<StateDeltaPayload> {
    let mut frames: Vec<StateDeltaPayload> = diff.upserts.into_iter().map(upsert).collect();
    frames.extend(diff.removed.into_iter().map(removed));
    frames.extend(diff.order.map(order));
    frames
}

pub fn diff_story_frames(
    previous: &[serde_json::Value],
    current: &[serde_json::Value],
) -> Vec<StateDeltaPayload> {
    map_frames(
        diff_keyed(previous, current),
        |story| StateDeltaPayload::StoryUpserted { story },
        |id| StateDeltaPayload::StoryRemoved { id },
        |ids| StateDeltaPayload::StoryOrder { ids },
    )
}
