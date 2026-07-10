use crate::message::{BROADCAST_ALL, Message};
use crate::routing::short_name;
use crate::store::Select;

pub(crate) fn stamp_delivery_ids(select: &Select, msg: &mut Message) {
    select.select(|state| {
        let routing = &state.routing;
        msg.to_ids = msg
            .to
            .as_ref()
            .map(|names| crate::store::routing::resolve_many(routing, names));
        msg.addressee_ids = match msg.addressee.as_deref() {
            None | Some(BROADCAST_ALL) => None,
            Some(target) => Some(crate::store::routing::resolve(routing, short_name(target))),
        };
        if msg.sender_id.is_none() {
            msg.sender_id = crate::store::routing::resolve(routing, short_name(&msg.from))
                .into_iter()
                .next();
        }
    });
}
