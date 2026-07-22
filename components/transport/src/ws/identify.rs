use std::sync::Arc;

use user_routing::short_name;

pub(super) fn resolve_routing_user_id(
    store: &Arc<store::Store>,
    dispatch: &store::Dispatch,
    identity: &str,
    identify_user_id: Option<String>,
) -> Option<String> {
    match identify_user_id {
        Some(user_id) => {
            let known = store
                .selector()
                .select(|state| store::routing::display_for(&state.routing, &user_id).is_some());
            if !known {
                dispatch.dispatch(
                    store::routing::RoutingAction::Registered {
                        user_id: user_id.clone(),
                        display: short_name(identity).to_string(),
                        role: None,
                    }
                    .into(),
                );
            }
            Some(user_id)
        }
        None => store
            .selector()
            .select(|state| store::routing::resolve(&state.routing, short_name(identity)))
            .into_iter()
            .next(),
    }
}

pub(super) fn roster_key_for(routing_user_id: &Option<String>, is_owner: bool) -> Option<String> {
    if is_owner {
        return None;
    }
    routing_user_id.clone()
}
