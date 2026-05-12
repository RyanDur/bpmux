#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

// Tests for #1301: boot-time markers-store → iteration-issue migration
// wire-up at lib.rs::migrate_markers_to_iteration_issues_at_boot.
// Substrate unit-tests live in session/migrate_markers.rs; this file
// pins the boot-time integration end-to-end: real session setup +
// real refresh-driver + assert markers migrate to iteration-issues
// with member.iteration_id populated.

use crate::board::RefreshDriver;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState, StoryType};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::store::BoardStore;
use crate::session::iteration_markers::{IterationMarker, IterationMarkersStore, save};
use crate::session::message::Message;
use crate::session::{BROADCAST_CHANNEL_CAPACITY, Session};
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use tokio::sync::broadcast;

fn seed_member(number: u64) -> Story {
    Story::new_minimal(
        number,
        format!("[Feature] member-{number}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch = SyncOrchestrator::new(store, adapter);
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch);
    orch.refresh_all().await.unwrap();
    orch
}

fn build_session(session_dir: std::path::PathBuf) -> Arc<Session> {
    let (wake_tx, _wake_rx) = broadcast::channel::<Message>(BROADCAST_CHANNEL_CAPACITY);
    Arc::new(Session::new(session_dir, "pm".to_string(), wake_tx))
}

#[tokio::test]
async fn boot_migration_creates_iteration_issues_for_markers_and_labels_members() {
    // Set up: 1 marker (Aquila) with 2 member-stories already in store.
    let session_tmp = tempfile::tempdir().unwrap();
    let session = build_session(session_tmp.path().to_path_buf());

    let markers = vec![IterationMarker {
        id: 2,
        theme_name: "Aquila".to_string(),
        position: 0,
        created_at: "2026-05-10T00:00:00Z".to_string(),
        stories: vec![100, 200],
    }];
    save(
        &session.dir,
        &IterationMarkersStore {
            markers,
            next_id: 3,
        },
    )
    .unwrap();

    let refresh = build_refresh(vec![seed_member(100), seed_member(200)]).await;

    // Fire the boot-time wire-up.
    crate::migrate_markers_to_iteration_issues_at_boot(session.clone(), refresh.clone()).await;

    // Assert: 1 iteration-issue created (story_type=Iteration, title
    // "Iteration Aquila"); both members' iteration_id populated.
    let iteration_issues: Vec<Story> = refresh
        .all_stories()
        .into_iter()
        .filter(|s| matches!(s.story_type, StoryType::Iteration))
        .collect();
    assert_eq!(iteration_issues.len(), 1, "1 iteration-issue created");
    let iter_issue = &iteration_issues[0];
    assert_eq!(iter_issue.title, "Iteration Aquila");
    let new_n = iter_issue.number;
    let m100 = refresh.story(100).expect("member 100 still exists");
    assert_eq!(
        m100.iteration_id,
        Some(new_n),
        "member 100 iteration_id populated"
    );
    let m200 = refresh.story(200).expect("member 200 still exists");
    assert_eq!(
        m200.iteration_id,
        Some(new_n),
        "member 200 iteration_id populated"
    );
}

#[tokio::test]
async fn boot_migration_is_noop_when_markers_store_empty() {
    let session_tmp = tempfile::tempdir().unwrap();
    let session = build_session(session_tmp.path().to_path_buf());
    let refresh = build_refresh(vec![]).await;
    crate::migrate_markers_to_iteration_issues_at_boot(session, refresh.clone()).await;
    let iteration_issues: Vec<Story> = refresh
        .all_stories()
        .into_iter()
        .filter(|s| matches!(s.story_type, StoryType::Iteration))
        .collect();
    assert_eq!(iteration_issues.len(), 0, "no-markers → no migration");
}

#[tokio::test]
async fn boot_migration_idempotent_when_iteration_issue_already_exists() {
    let session_tmp = tempfile::tempdir().unwrap();
    let session = build_session(session_tmp.path().to_path_buf());
    save(
        &session.dir,
        &IterationMarkersStore {
            markers: vec![IterationMarker {
                id: 2,
                theme_name: "Aquila".to_string(),
                position: 0,
                created_at: "2026-05-10T00:00:00Z".to_string(),
                stories: vec![],
            }],
            next_id: 3,
        },
    )
    .unwrap();
    // Seed an existing "Iteration Aquila" iteration-issue (post-prior-
    // boot state).
    let mut existing = Story::new_minimal(
        1297,
        "Iteration Aquila".to_string(),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    existing.story_type = StoryType::Iteration;
    existing.raw_labels = vec!["iteration".to_string()];
    let refresh = build_refresh(vec![existing]).await;
    crate::migrate_markers_to_iteration_issues_at_boot(session, refresh.clone()).await;
    // Second-run produces all-Skip → only the seeded #1297 should
    // remain; no duplicate iteration-issue.
    let iteration_issues: Vec<Story> = refresh
        .all_stories()
        .into_iter()
        .filter(|s| matches!(s.story_type, StoryType::Iteration))
        .collect();
    assert_eq!(
        iteration_issues.len(),
        1,
        "idempotent: skip when matching iteration-issue already exists"
    );
    assert_eq!(iteration_issues[0].number, 1297);
}
