#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{StoryId, Work};
use crate::sync::{QueuedEvent, load_outbox_from_disk};

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 5, 5, 22, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

#[tokio::test]
async fn outbox_round_trips_the_only_shape_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new-outbox.json");
    let queued = QueuedEvent::new(StoryEvent::new(
        meta(),
        StoryId::new(7),
        StoryEventKind::StateTransitioned {
            new_state: Work::Accepted,
            placement: None,
        },
    ));
    let payload = serde_json::to_vec(&vec![queued]).unwrap();
    std::fs::write(&path, payload).unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = entries.front().unwrap();
    assert_eq!(entry.event.story_id(), StoryId::new(7));
    assert!(matches!(
        entry.event.kind,
        StoryEventKind::StateTransitioned {
            new_state: Work::Accepted,
            ..
        }
    ));
}
