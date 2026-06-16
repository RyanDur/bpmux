use std::sync::Arc;

use super::short_name;

pub(crate) fn is_reserved_display(name: &str) -> bool {
    use crate::orchestrator::stores::RosterRole;
    matches!(role_from_short_name(short_name(name)), RosterRole::Owner)
}

pub(crate) fn role_from_short_name(short: &str) -> crate::orchestrator::stores::RosterRole {
    use crate::orchestrator::stores::RosterRole;
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

async fn dispatch_roster_upsert(
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    routing_id: &str,
    identity: &str,
    presence: crate::orchestrator::stores::Presence,
) {
    use crate::orchestrator::OrchestratorAction;
    use crate::orchestrator::stores::RosterAction;
    let short = short_name(identity).to_string();
    let role = role_from_short_name(&short);
    orchestrator
        .dispatch(OrchestratorAction::Roster(
            RosterAction::RosterMemberUpserted {
                routing_id: routing_id.to_string(),
                name: short,
                identity: identity.to_string(),
                role,
                presence,
            },
        ))
        .await;
}

pub(crate) async fn dispatch_roster_join(
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    routing_id: &str,
    identity: &str,
) {
    use crate::orchestrator::stores::Presence;
    dispatch_roster_upsert(orchestrator, routing_id, identity, Presence::Online).await;
}

pub(crate) async fn dispatch_roster_rederived(
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    routing_id: &str,
    identity: &str,
) {
    use crate::orchestrator::stores::Presence;
    dispatch_roster_upsert(orchestrator, routing_id, identity, Presence::Offline).await;
}

pub(crate) async fn dispatch_roster_leave(
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    routing_id: &str,
) {
    use crate::orchestrator::OrchestratorAction;
    use crate::orchestrator::stores::RosterAction;
    orchestrator
        .dispatch(OrchestratorAction::Roster(
            RosterAction::RosterMemberRemoved {
                routing_id: routing_id.to_string(),
            },
        ))
        .await;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn is_reserved_display_blocks_the_owner_identity_but_allows_agent_roles() {
        assert!(
            is_reserved_display("ryan"),
            "an agent must never be able to present as the owner — set_name/register to the owner identity is rejected (the owner is a human, never an agent)"
        );
        assert!(
            is_reserved_display("ryan:electron"),
            "the reservation is short-name based — a full identity still resolves to the owner role"
        );
        assert!(
            !is_reserved_display("dev"),
            "agent roles (dev/pm/designer) are LEGITIMATE names the real agents use — peer-vs-peer name collisions are a display/#1714 concern, not an owner-forgery this blocks"
        );
        assert!(!is_reserved_display("pm") && !is_reserved_display("designer"));
    }

    fn fresh_orchestrator()
    -> Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>
    {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let (tx, _rx) = tokio::sync::broadcast::channel::<crate::session::Message>(64);
        let session = Arc::new(crate::session::Session::new(
            dir.path().to_path_buf(),
            "local".to_string(),
            tx,
        ));
        std::mem::forget(dir);
        Arc::new(crate::orchestrator::Orchestrator::new(
            session,
            None,
            crate::transport::ci_state_store::CiStateStore::new(),
            tokio_util::sync::CancellationToken::new(),
        ))
    }

    #[tokio::test]
    async fn a_rederived_member_is_offline_until_a_reconnect_reconfirms_it_online() {
        use crate::orchestrator::stores::Presence;
        let orch = fresh_orchestrator();

        dispatch_roster_rederived(&orch, "sess-dev", "dev").await;
        let members = orch.roster_store().members();
        assert_eq!(
            members.len(),
            1,
            "the startup re-derive re-represents the registered agent recovered from the persisted store"
        );
        assert_eq!(members[0].name, "dev");
        assert_eq!(
            members[0].presence,
            Presence::Offline,
            "a roster entry RE-DERIVED from the persisted RoutingStore on a server-restart is registered-but-UNCONFIRMED — it must NOT be assumed live (Online). Re-deriving everyone as Online would make a since-dead agent show live indefinitely; Offline is the honest placeholder, reconfirmed only when the live agent reconnects (#1745 stale-guard)"
        );

        dispatch_roster_join(&orch, "sess-dev", "dev").await;
        let reconfirmed = orch.roster_store().members();
        assert_eq!(
            reconfirmed[0].presence,
            Presence::Online,
            "the live agent's reconnect (ws roster_join) upserts the SAME routing-id to Online — the re-derived placeholder self-heals to live on reconfirmation, with no manual re-register"
        );
    }
}
