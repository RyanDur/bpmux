use std::sync::Arc;

pub(crate) async fn dispatch_roster_join(
    store: &Arc<crate::store::Store>,
    routing_id: &str,
    identity: &str,
) {
    use crate::store::AppAction;
    use crate::store::{Presence, RosterAction};
    store
        .dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
            routing_id: routing_id.to_string(),
            identity: identity.to_string(),
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
