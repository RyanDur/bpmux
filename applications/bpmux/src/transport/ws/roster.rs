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

pub(crate) async fn dispatch_roster_join(
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    identity: &str,
) {
    use crate::orchestrator::OrchestratorAction;
    use crate::orchestrator::stores::{Presence, RosterAction};
    let short = short_name(identity).to_string();
    let role = role_from_short_name(&short);
    orchestrator
        .dispatch(OrchestratorAction::Roster(
            RosterAction::RosterMemberUpserted {
                name: short,
                identity: identity.to_string(),
                role,
                presence: Presence::Online,
            },
        ))
        .await;
}

pub(crate) async fn dispatch_roster_leave(
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    identity: &str,
) {
    use crate::orchestrator::OrchestratorAction;
    use crate::orchestrator::stores::RosterAction;
    let short = short_name(identity).to_string();
    orchestrator
        .dispatch(OrchestratorAction::Roster(
            RosterAction::RosterMemberRemoved { name: short },
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
}
