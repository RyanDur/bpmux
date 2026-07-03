use std::sync::Arc;

use crate::routing::short_name;

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
    use crate::store::AppAction;
    use crate::store::{Presence, RosterAction};
    let short = short_name(identity).to_string();
    let role = role_from_short_name(&short);
    store
        .dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
            routing_id: routing_id.to_string(),
            name: short,
            identity: identity.to_string(),
            role,
            presence: Presence::Online,
        }))
        .await;
}

pub(crate) async fn dispatch_roster_leave(store: &Arc<crate::store::Store>, routing_id: &str) {
    use crate::store::AppAction;
    use crate::store::RosterAction;
    store
        .dispatch(AppAction::Roster(RosterAction::RosterMemberRemoved {
            routing_id: routing_id.to_string(),
        }))
        .await;
}

pub(crate) async fn dispatch_roster_presence(
    store: &Arc<crate::store::Store>,
    routing_id: &str,
    presence: crate::store::Presence,
) {
    use crate::store::AppAction;
    use crate::store::RosterAction;
    store
        .dispatch(AppAction::Roster(RosterAction::RosterPresenceChanged {
            routing_id: routing_id.to_string(),
            presence,
        }))
        .await;
}
