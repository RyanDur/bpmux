#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use super::SqliteStorage;
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::{StoryId, Work};
use source_sync::OutboxRepo;
use source_sync::QueuedEvent;

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        actor_id: None,
        ts: Utc.with_ymd_and_hms(2026, 5, 5, 22, 0, 0).unwrap(),
    }
}

#[test]
fn outbox_round_trips_the_only_shape_through_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();
    let queued = QueuedEvent::new(StoryEvent::new(
        meta(),
        StoryId::new(7),
        StoryEventKind::StateTransitioned {
            new_state: Work::Accepted,
            placement: None,
        },
    ));
    storage.save_outbox(&[queued]).unwrap();

    let entries = storage.load_outbox().unwrap();
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
