#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use tempfile::TempDir;
use tokio::sync::broadcast;

use super::evidence::EvidencePersister;
use crate::hub::Hub;
use crate::product_management::evidence::{
    EvidenceAction, evidence_for_story, evidence_path, from_store, load, to_store,
};
use crate::product_management::story::StoryId;
use crate::store::{AppAction, AppState, Presence, RosterAction, app_reduce};
use kernel::{Dispatch, Listener, Reaction};
use message::Message;

fn fresh_hub() -> (Arc<Hub>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    (hub, dir)
}

fn no_op_dispatch() -> Dispatch<AppAction> {
    Arc::new(|_action: AppAction| {})
}

fn verdict_action(story_id: u64, role: &str) -> AppAction {
    AppAction::Evidence(EvidenceAction::VerdictSubmitted {
        story_id: StoryId::new(story_id),
        from_role: role.to_string(),
        verdict: "verified".to_string(),
        substrate_grounding: "src/foo.rs cite".to_string(),
        ts: chrono::Utc::now(),
        grounded_head: Some("a-frozen-sha".to_string()),
    })
}

fn roster_action() -> AppAction {
    AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-dev".to_string(),
        identity: "dev:claude-code".to_string(),
        presence: Presence::Online,
    })
}

async fn drive(
    listener: &dyn Listener<AppState, AppAction>,
    previous: AppState,
    action: &AppAction,
) {
    let current = app_reduce(&previous, action);
    let reaction = Reaction {
        action: action.clone(),
        previous: Arc::new(previous),
        current: Arc::new(current),
        dispatch: no_op_dispatch(),
    };
    listener.on_action(reaction).await;
}

#[test]
fn a_dispatched_verdict_is_visible_to_the_selector_with_no_file_present() {
    let (hub, _dir) = fresh_hub();
    let state = app_reduce(&AppState::default(), &verdict_action(42, "designer"));
    assert_eq!(
        evidence_for_story(&state.evidence, StoryId::new(42)).len(),
        1,
        "the gate reads the store selector, so a dispatched verdict is present synchronously"
    );
    assert!(
        !evidence_path(&hub.dir).exists(),
        "no persister ran, so verdicts.json is absent — the store is the source, the file lags"
    );
}

#[tokio::test]
async fn a_verdict_is_flushed_to_disk_by_the_persister() {
    let (hub, _dir) = fresh_hub();
    let listener = EvidencePersister::new(hub.clone());
    drive(
        &listener,
        AppState::default(),
        &verdict_action(42, "designer"),
    )
    .await;
    let hydrated = from_store(&load(&hub.dir));
    let listed = evidence_for_story(&hydrated, StoryId::new(42));
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].grounded_head.as_deref(), Some("a-frozen-sha"));
}

#[tokio::test]
async fn an_untouched_evidence_grain_writes_no_file() {
    let (hub, _dir) = fresh_hub();
    let listener = EvidencePersister::new(hub.clone());
    drive(&listener, AppState::default(), &roster_action()).await;
    assert!(
        !evidence_path(&hub.dir).exists(),
        "the pointer guard keeps the persister quiet when evidence is untouched"
    );
}

#[test]
fn a_cold_boot_hydrates_the_evidence_grain_from_its_file() {
    let (hub, _dir) = fresh_hub();
    let seeded = app_reduce(&AppState::default(), &verdict_action(42, "designer"));
    crate::product_management::evidence::save(&hub.dir, &to_store(&seeded.evidence)).unwrap();

    let state = AppState::default().with_evidence(from_store(&load(&hub.dir)));

    assert_eq!(
        evidence_for_story(&state.evidence, StoryId::new(42)),
        evidence_for_story(&seeded.evidence, StoryId::new(42)),
        "a cold boot restores the evidence grain from verdicts.json"
    );
}
