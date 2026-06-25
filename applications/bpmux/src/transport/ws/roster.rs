use std::sync::Arc;

use super::short_name;

pub(crate) fn is_reserved_display(name: &str) -> bool {
    use crate::store::RosterRole;
    matches!(role_from_short_name(short_name(name)), RosterRole::Owner)
}

pub(crate) fn role_from_short_name(short: &str) -> crate::store::RosterRole {
    use crate::store::RosterRole;
    let workflow = crate::workflow::current_workflow();
    let purpose = workflow
        .role
        .iter()
        .find(|r| r.id == short)
        .and_then(|r| r.purposes.first().map(String::as_str));
    match purpose {
        Some("designer") => RosterRole::Designer,
        Some("dev") => RosterRole::Dev,
        Some("pm") => RosterRole::Pm,
        Some("owner") => RosterRole::Owner,
        _ => RosterRole::Other,
    }
}

pub(crate) async fn dispatch_roster_join(
    store: &Arc<crate::store::Store>,
    routing_id: &str,
    identity: &str,
) {
    use crate::store::StoreAction;
    use crate::store::{Presence, RosterAction};
    let short = short_name(identity).to_string();
    let role = role_from_short_name(&short);
    store
        .dispatch(StoreAction::Roster(RosterAction::RosterMemberUpserted {
            routing_id: routing_id.to_string(),
            name: short,
            identity: identity.to_string(),
            role,
            presence: Presence::Online,
        }))
        .await;
}

pub(crate) async fn dispatch_roster_leave(store: &Arc<crate::store::Store>, routing_id: &str) {
    use crate::store::RosterAction;
    use crate::store::StoreAction;
    store
        .dispatch(StoreAction::Roster(RosterAction::RosterMemberRemoved {
            routing_id: routing_id.to_string(),
        }))
        .await;
}

pub(crate) async fn dispatch_roster_presence(
    store: &Arc<crate::store::Store>,
    routing_id: &str,
    presence: crate::store::Presence,
) {
    use crate::store::RosterAction;
    use crate::store::StoreAction;
    store
        .dispatch(StoreAction::Roster(RosterAction::RosterPresenceChanged {
            routing_id: routing_id.to_string(),
            presence,
        }))
        .await;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn is_reserved_display_blocks_the_owner_identity_but_allows_agent_roles() {
        assert!(
            is_reserved_display("owner"),
            "an agent must never be able to present as the owner — set_name/register to the owner identity is rejected (the owner is a human, never an agent)"
        );
        assert!(
            is_reserved_display("owner:electron"),
            "the reservation is short-name based — a full identity still resolves to the owner role"
        );
        assert!(
            !is_reserved_display("dev"),
            "agent roles (dev/pm/designer) are LEGITIMATE names the real agents use — peer-vs-peer name collisions are a display/#1714 concern, not an owner-forgery this blocks"
        );
        assert!(!is_reserved_display("pm") && !is_reserved_display("designer"));
    }

    fn fresh_orchestrator() -> Arc<crate::store::Store> {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let (tx, _rx) = tokio::sync::broadcast::channel::<crate::message::Message>(64);
        let session = Arc::new(crate::session::Session::new(
            dir.path().to_path_buf(),
            "local".to_string(),
            tx,
        ));
        std::mem::forget(dir);
        Arc::new(crate::store::Store::new(
            session,
            None,
            crate::transport::ci_state_store::CiStateStore::new(),
            tokio_util::sync::CancellationToken::new(),
        ))
    }

    #[tokio::test]
    async fn an_agent_pushes_busy_idle_help_setting_its_own_roster_presence() {
        use crate::store::Presence;
        let orch = fresh_orchestrator();
        dispatch_roster_join(&orch, "sess-dev", "dev").await;

        dispatch_roster_presence(&orch, "sess-dev", Presence::Busy).await;
        assert_eq!(
            orch.roster_store().members()[0].presence,
            Presence::Busy,
            "an agent pushing busy sets its OWN roster presence — the agent-driven colour-source #1739 adds (the dot reads roster Presence)"
        );

        dispatch_roster_presence(&orch, "sess-dev", Presence::Help).await;
        assert_eq!(
            orch.roster_store().members()[0].presence,
            Presence::Help,
            "help is reachable BY THE AGENT — previously unsettable (owner-park set only Blocked); pushing help closes the Help-never-set gap"
        );

        dispatch_roster_presence(&orch, "sess-dev", Presence::Idle).await;
        assert_eq!(
            orch.roster_store().members()[0].presence,
            Presence::Idle,
            "idle (the Stop-hook's definitive turn-end) is pushable, winning over any prior state"
        );
    }
}
