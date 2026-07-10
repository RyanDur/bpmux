use crate::message::{BROADCAST_ALL, Message};
use crate::routing::short_name;
use crate::store::Select;
use crate::store::routing::RoutingState;

pub(crate) fn stamp_delivery_ids(select: &Select, msg: &mut Message) {
    select.select(|state| {
        let routing = &state.routing;
        msg.to_ids = msg
            .to
            .as_deref()
            .and_then(|names| complete_resolution(routing, names));
        msg.addressee_ids = match msg.addressee.as_deref() {
            None | Some(BROADCAST_ALL) => None,
            Some(target) => non_empty(crate::store::routing::resolve(routing, short_name(target))),
        };
        if msg.sender_id.is_none() {
            msg.sender_id = crate::store::routing::resolve(routing, short_name(&msg.from))
                .into_iter()
                .next();
        }
    });
}

fn complete_resolution(routing: &RoutingState, names: &[String]) -> Option<Vec<String>> {
    let mut seen = std::collections::HashSet::new();
    let mut ids = Vec::new();
    for name in names {
        if name == BROADCAST_ALL {
            continue;
        }
        let resolved = crate::store::routing::resolve(routing, short_name(name));
        if resolved.is_empty() {
            return None;
        }
        for id in resolved {
            if seen.insert(id.clone()) {
                ids.push(id);
            }
        }
    }
    non_empty(ids)
}

fn non_empty(ids: Vec<String>) -> Option<Vec<String>> {
    if ids.is_empty() { None } else { Some(ids) }
}
