#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use tempfile::TempDir;

use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::kernel::{Dispatch, Listener, Reaction};
use crate::product_management::story::StoryId;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::store::app::{AppAction, AppState};
use crate::sync::{MutationId, QueuedWrite, load_outbox_from_disk};

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

fn reaction(previous: Arc<AppState>, current: Arc<AppState>) -> Reaction<AppState, AppAction> {
    let dispatch: Dispatch<AppAction> = Arc::new(|_action: AppAction| {});
    Reaction {
        action: AppAction::Sync(SyncAction::WritePushed { id: MutationId(0) }),
        previous,
        current,
        dispatch,
    }
}

#[tokio::test]
async fn a_stale_snapshot_never_overwrites_a_newer_one_on_disk() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("board-outbox.json");
    let persister = OutboxPersister::new(OutboxSink::Path(path.clone()));

    let with_one = Arc::new(AppState::default().with_outbox(vec![one_write()]));

    let first = persister.on_action(reaction(
        Arc::new(AppState::default()),
        Arc::clone(&with_one),
    ));
    let second = persister.on_action(reaction(with_one, Arc::new(AppState::default())));

    second.await;
    first.await;

    let events = load_outbox_from_disk(&path).expect("outbox loads");
    assert!(
        events.is_empty(),
        "the later empty-queue snapshot wins; a stale earlier save cannot clobber it"
    );
}
