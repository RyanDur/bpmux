#![allow(clippy::unwrap_used)]

use crate::session::{BROADCAST_CHANNEL_CAPACITY, Session};
use tempfile::tempdir;
use tokio::sync::broadcast;

fn fresh_session() -> Session {
    let dir = tempdir().unwrap().keep();
    let (wake_tx, _wake_rx) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
    Session::new(dir, "owner".to_string(), wake_tx)
}

#[test]
fn fresh_session_has_empty_roster() {
    let session = fresh_session();
    let snapshot = session.roster_snapshot();
    assert!(snapshot.is_empty());
}

#[test]
fn session_pre_populates_roster_from_seed_file_at_session_dir() {
    let dir = tempdir().unwrap().keep();
    let seed = vec!["dev:claude-code".to_string(), "pm:claude-code".to_string()];
    let path = dir.join("roster_seed.json");
    std::fs::write(&path, serde_json::to_string(&seed).unwrap()).unwrap();
    let (wake_tx, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
    let session = Session::new(dir, "owner".to_string(), wake_tx);
    let snapshot = session.roster_snapshot();
    assert_eq!(
        snapshot,
        vec!["dev:claude-code".to_string(), "pm:claude-code".to_string()]
    );
}

#[test]
fn roster_join_populates_roster() {
    let session = fresh_session();
    session.roster_join("dev:claude-code");
    let snapshot = session.roster_snapshot();
    assert_eq!(snapshot, vec!["dev:claude-code".to_string()]);
}

#[test]
fn roster_leave_removes_entry() {
    let session = fresh_session();
    session.roster_join("dev:claude-code");
    session.roster_join("pm:claude-code");
    session.roster_leave("dev:claude-code");
    let snapshot = session.roster_snapshot();
    assert_eq!(snapshot, vec!["pm:claude-code".to_string()]);
}

#[test]
fn cloned_session_starts_with_empty_roster() {
    let original = fresh_session();
    original.roster_join("dev:claude-code");
    original.roster_join("pm:claude-code");
    let cloned = original.clone();
    let snapshot = cloned.roster_snapshot();
    assert!(
        snapshot.is_empty(),
        "Clone is a snapshot of the log, not live peers — roster must start fresh per Session::clone semantics"
    );
}

#[test]
fn roster_join_is_idempotent() {
    let session = fresh_session();
    session.roster_join("dev:claude-code");
    session.roster_join("dev:claude-code");
    let snapshot = session.roster_snapshot();
    assert_eq!(snapshot.len(), 1);
}

#[test]
fn reconcile_drops_a_ghost_agent_with_no_live_terminal_but_keeps_role_and_live_bare_agent() {
    use crate::routing::bridge_agent_name;

    let live_terminal = "aaaaaaaa-1111-2222-3333-444455556666";
    let dead_terminal = "deadbeef-9999-8888-7777-666655554444";

    let session = fresh_session();
    session.roster_join("dev:claude-code");
    session.roster_join(&bridge_agent_name(live_terminal));
    session.roster_join(&bridge_agent_name(dead_terminal));

    session.reconcile_roster(&[live_terminal.to_string()]);

    let snapshot = session.roster_snapshot();
    assert!(
        !snapshot.contains(&bridge_agent_name(dead_terminal)),
        "the ghost agent-<id> whose terminal is GONE from the live set is reaped — this is the orphaned sidebar entry that survived a restart/reclaim with no liveness reconcile; got {snapshot:?}"
    );
    assert!(
        snapshot.contains(&"dev:claude-code".to_string()),
        "a claimed role identity (no agent- prefix) is never a reconcile target — only auto-assigned agent-<id> names are reconciled; got {snapshot:?}"
    );
    assert!(
        snapshot.contains(&bridge_agent_name(live_terminal)),
        "a LEGITIMATE live bare-agent (its terminal IS in the live set) keeps its agent-<id> name — the discriminator is terminal-liveness, so the #1656 agnostic bare-agents are never reaped; got {snapshot:?}"
    );
}

#[test]
fn roster_snapshot_is_sorted() {
    let session = fresh_session();
    session.roster_join("zeta");
    session.roster_join("alpha");
    session.roster_join("mu");
    let snapshot = session.roster_snapshot();
    assert_eq!(snapshot, vec!["alpha", "mu", "zeta"]);
}
