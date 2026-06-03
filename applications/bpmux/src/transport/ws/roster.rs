use std::sync::Arc;

use super::short_name;

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
