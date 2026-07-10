use std::collections::HashSet;

use crate::routing::cursor_time::to_millis;
use crate::routing::short_name;

use super::RoutingState;

pub struct RoutingReconcile {
    pub upserts: Vec<(String, String, String)>,
    pub cursors: Vec<(String, i64)>,
    pub deletes: Vec<String>,
}

pub fn reconcile(previous: &RoutingState, current: &RoutingState) -> RoutingReconcile {
    let prev = &previous.identity;
    let cur = &current.identity;
    let mut upserts = Vec::new();
    let mut cursors = Vec::new();
    for id in cur.all_ids() {
        if let Some(display) = cur.display_for(&id)
            && prev.display_for(&id).as_deref() != Some(display.as_str())
        {
            let role = cur
                .role_for(&id)
                .unwrap_or_else(|| short_name(&display).to_string());
            upserts.push((id.clone(), display.clone(), role));
        }
        let current_cursor = cur.cursor(&id);
        if current_cursor != prev.cursor(&id)
            && let Some(ms) = current_cursor.as_deref().and_then(to_millis)
        {
            cursors.push((id, ms));
        }
    }
    let live: HashSet<String> = cur.all_ids().into_iter().collect();
    let deletes = prev
        .all_ids()
        .into_iter()
        .filter(|id| !live.contains(id))
        .collect();
    RoutingReconcile {
        upserts,
        cursors,
        deletes,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::reconcile;
    use crate::routing::slice::{RoutingAction, RoutingState, reduce};

    fn apply(state: &RoutingState, action: RoutingAction) -> RoutingState {
        reduce(state, &action).unwrap_or_else(|| state.clone())
    }

    fn registered(
        state: &RoutingState,
        user_id: &str,
        display: &str,
        role: Option<&str>,
    ) -> RoutingState {
        apply(
            state,
            RoutingAction::Registered {
                user_id: user_id.to_string(),
                display: display.to_string(),
                role: role.map(str::to_string),
            },
        )
    }

    fn instant(rfc3339: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .expect("rfc3339")
            .timestamp_millis()
    }

    #[test]
    fn a_new_registration_upserts_the_display_and_the_assigned_role_never_laundered_from_it() {
        let current = registered(&RoutingState::default(), "sess-dev", "aria", Some("dev"));
        let plan = reconcile(&RoutingState::default(), &current);
        assert_eq!(
            plan.upserts,
            vec![(
                "sess-dev".to_string(),
                "aria".to_string(),
                "dev".to_string()
            )],
            "a new session upserts (id, opaque display, ASSIGNED role) — the display 'aria' carries no workflow-role prefix, so a short_name(display) laundering would have produced 'aria', not 'dev'"
        );
        assert!(
            plan.cursors.is_empty(),
            "a freshly-registered session has an empty cursor — it never parses to millis, so no last_delivered write"
        );
        assert!(plan.deletes.is_empty());
    }

    #[test]
    fn a_new_registration_with_no_role_falls_back_to_short_name_display_the_skew_only_compat_path()
    {
        let current = registered(
            &RoutingState::default(),
            "sess-dev",
            "dev:claude-code",
            None,
        );
        let plan = reconcile(&RoutingState::default(), &current);
        assert_eq!(
            plan.upserts,
            vec![(
                "sess-dev".to_string(),
                "dev:claude-code".to_string(),
                "dev".to_string()
            )],
            "an old-binary registration carrying no role falls back to short_name(display) — value-identical to pre-C3 behavior, retiring with B-5"
        );
    }

    #[test]
    fn a_cursor_advance_writes_only_the_last_delivered_millis_for_the_changed_session() {
        let previous = registered(&RoutingState::default(), "sess-dev", "dev", None);
        let current = apply(
            &previous,
            RoutingAction::CursorAdvanced {
                user_id: "sess-dev".to_string(),
                cursor: "2026-06-11T20:00:05Z".to_string(),
            },
        );
        let plan = reconcile(&previous, &current);
        assert!(
            plan.upserts.is_empty(),
            "the display did not change across a cursor advance — no redundant upsert"
        );
        assert_eq!(
            plan.cursors,
            vec![("sess-dev".to_string(), instant("2026-06-11T20:00:05Z"))],
            "only the cursor-changed session writes last_delivered, as an instant"
        );
    }

    #[test]
    fn a_rename_upserts_the_new_display_but_the_role_assignment_survives_untouched() {
        let previous = registered(&RoutingState::default(), "sess-1", "dev", Some("dev"));
        let current = apply(
            &previous,
            RoutingAction::Renamed {
                user_id: "sess-1".to_string(),
                display: "dev-renamed".to_string(),
            },
        );
        let plan = reconcile(&previous, &current);
        assert_eq!(
            plan.upserts,
            vec![(
                "sess-1".to_string(),
                "dev-renamed".to_string(),
                "dev".to_string()
            )],
            "a rename upserts the new DISPLAY but the persisted role stays the ASSIGNMENT ('dev') — a display-derivation would have relaundered it to 'dev-renamed', which is exactly the conflation C-3 retires"
        );
        assert!(plan.cursors.is_empty());
    }

    #[test]
    fn a_reap_deletes_the_removed_session_rows() {
        let previous = registered(&RoutingState::default(), "dev-stale", "dev", None);
        let previous = registered(&previous, "dev-live", "dev", None);
        let previous = apply(
            &previous,
            RoutingAction::TerminalLinked {
                user_id: "dev-live".to_string(),
                terminal_id: "term-live".to_string(),
            },
        );
        let current = apply(
            &previous,
            RoutingAction::Reaped {
                display: "dev".to_string(),
            },
        );
        let plan = reconcile(&previous, &current);
        assert_eq!(
            plan.deletes,
            vec!["dev-stale".to_string()],
            "the reaped unclaimed duplicate is deleted; the live terminal-bound id survives"
        );
    }
}
