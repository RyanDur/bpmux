#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use chrono::{TimeZone, Utc};

use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::product_management::story::StoryId;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::store::app::{AppAction, AppState, app_reduce};
use crate::store::test_support::eventually;
use crate::sync::{MutationId, QueuedEvent, QueuedWrite};

use super::{OutboxPersister, OutboxSink, SyncAction};

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 7, 8, 0, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn one_write() -> QueuedWrite {
    QueuedWrite::from_event(
        &GhLabelCodec,
        MutationId(1),
        StoryEvent::new(
            meta(),
            StoryId::new(1),
            StoryEventKind::TagAdded {
                label: "ready".to_string(),
            },
        ),
    )
}

#[tokio::test]
async fn a_rapid_sync_burst_persists_only_the_latest_snapshot_never_the_stale_intermediate() {
    let saves: Arc<Mutex<Vec<Vec<QueuedEvent>>>> = Arc::new(Mutex::new(Vec::new()));
    let store = crate::kernel::Store::new(AppState::default(), Box::new(app_reduce));
    store.add_listener(Arc::new(OutboxPersister::new(OutboxSink::Recording(
        Arc::clone(&saves),
    ))));

    let write = one_write();
    let id = write.id;
    store.dispatch(AppAction::Sync(SyncAction::WriteQueued { write }));
    store.dispatch(AppAction::Sync(SyncAction::WritePushed { id }));

    eventually("the coalesced burst reaches the outbox sink", || {
        !saves.lock().unwrap().is_empty()
    })
    .await;
    tokio::task::yield_now().await;

    let saves = saves.lock().unwrap();
    assert_eq!(
        saves.len(),
        1,
        "under LATEST-STATE the two rapid sync changes coalesce: the persister saves exactly once, so a stale earlier snapshot can never overwrite a newer one; got {saves:?}"
    );
    assert!(
        saves[0].is_empty(),
        "the surviving save is the LATEST snapshot (the drained, empty queue) — the intermediate non-empty snapshot is discarded before it can reach disk"
    );
}
