use crate::routing::cursor_time::from_millis;
use crate::routing::{RoutingIdentity, UserRecord};

use super::RoutingState;

pub fn from_users(records: &[UserRecord]) -> RoutingState {
    let mut identity = RoutingIdentity::new();
    for record in records {
        identity.register(&record.session_id, &record.name);
        if let Some(cursor) = record.last_delivered_at.and_then(from_millis) {
            identity.advance_cursor(&record.session_id, &cursor);
        }
    }
    RoutingState { identity }
}
