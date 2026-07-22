use std::sync::Arc;

pub async fn dispatch_roster_join(store: &Arc<store::Store>, routing_id: &str, identity: &str) {
    use store::AppAction;
    use store::{Presence, RosterAction};
    store
        .dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
            routing_id: routing_id.to_string(),
            identity: identity.to_string(),
            presence: Presence::Online,
        }))
        .await;
}

pub async fn dispatch_roster_leave(store: &Arc<store::Store>, routing_id: &str) {
    use store::AppAction;
    use store::RosterAction;
    store
        .dispatch(AppAction::Roster(RosterAction::RosterMemberRemoved {
            routing_id: routing_id.to_string(),
        }))
        .await;
}

pub async fn dispatch_roster_presence(
    store: &Arc<store::Store>,
    routing_id: &str,
    presence: store::Presence,
) {
    use store::AppAction;
    use store::RosterAction;
    store
        .dispatch(AppAction::Roster(RosterAction::RosterPresenceChanged {
            routing_id: routing_id.to_string(),
            presence,
        }))
        .await;
}
