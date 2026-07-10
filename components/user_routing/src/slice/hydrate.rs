use crate::cursor_time::from_millis;
use crate::{RoutingIdentity, UserRecord};

use super::RoutingState;

pub fn from_users(records: &[UserRecord]) -> RoutingState {
    let mut identity = RoutingIdentity::new();
    for record in records {
        identity.register(&record.user_id, &record.name, &record.role);
        if let Some(cursor) = record.last_delivered_at.and_then(from_millis) {
            identity.advance_cursor(&record.user_id, &cursor);
        }
    }
    RoutingState { identity }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::from_users;
    use crate::UserRecord;

    fn record(user_id: &str, name: &str, role: &str) -> UserRecord {
        UserRecord {
            user_id: user_id.to_string(),
            name: name.to_string(),
            role: role.to_string(),
            kind: crate::default_kind(),
            last_delivered_at: None,
        }
    }

    #[test]
    fn hydrate_loads_the_persisted_role_column_as_the_assignment_not_a_display_re_derivation() {
        let state = from_users(&[record("sess-aria", "aria", "dev")]);
        assert_eq!(
            super::super::role_for(&state, "sess-aria"),
            Some("dev".to_string()),
            "the assignment persisted in the role column hydrates verbatim — display 'aria' carries no workflow-role prefix, so a display-derivation would have produced something else entirely"
        );
    }

    #[test]
    fn hydrate_of_a_legacy_row_carries_its_short_name_display_value_unchanged_no_migration() {
        let state = from_users(&[record("sess-dev", "dev:claude-code", "dev")]);
        assert_eq!(
            super::super::role_for(&state, "sess-dev"),
            Some("dev".to_string()),
            "a pre-C3 row's role column already holds short_name(display) — hydrate loads it as-is, the compat path: value-identical to the old derivation, no backfill/migration needed"
        );
    }
}
