use std::time::Duration;

use crate::orchestrator::stores::{Presence, RosterMember};
use crate::room::LivePtyRegistry;
use crate::session::routing_identity::RoutingStore;

pub const BUSY_WINDOW: Duration = Duration::from_secs(7);

pub fn presence_from_quiet(quiet_for: Duration) -> Presence {
    if quiet_for < BUSY_WINDOW {
        Presence::Busy
    } else {
        Presence::Idle
    }
}

pub fn terminal_quiet_for_id(
    routing: &RoutingStore,
    registry: &LivePtyRegistry,
    session_id: &str,
) -> Option<Duration> {
    routing
        .terminal_for(session_id)
        .and_then(|terminal_id| registry.quiet_for(&terminal_id))
}

pub fn activity_presence_changes(
    roster: &[RosterMember],
    quiet_for: impl Fn(&str) -> Option<Duration>,
) -> Vec<(String, Presence)> {
    let mut changes = Vec::new();
    for member in roster {
        if matches!(
            member.presence,
            Presence::Blocked | Presence::Help | Presence::Offline
        ) {
            continue;
        }
        let Some(quiet) = quiet_for(&member.routing_id) else {
            continue;
        };
        let next = presence_from_quiet(quiet);
        if next != member.presence {
            changes.push((member.routing_id.clone(), next));
        }
    }
    changes
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::orchestrator::stores::RosterRole;

    #[test]
    fn recent_terminal_output_reads_busy() {
        assert_eq!(
            presence_from_quiet(Duration::from_millis(0)),
            Presence::Busy,
            "terminal output within the busy-window is an actively-working agent — incl thinking, which IS the output-stream"
        );
    }

    #[test]
    fn a_micro_pause_within_the_busy_window_stays_busy() {
        assert_eq!(
            presence_from_quiet(BUSY_WINDOW - Duration::from_secs(1)),
            Presence::Busy,
            "an output-gap shorter than the busy-window stays BUSY — a micro-pause between status-line repaints is not a finished turn"
        );
    }

    #[test]
    fn quiet_past_the_window_reads_the_confident_green_idle() {
        assert_eq!(
            presence_from_quiet(BUSY_WINDOW + Duration::from_secs(1)),
            Presence::Idle,
            "sustained terminal-quiet is the agent sitting at its idle input prompt = genuinely free = the real confident green IDLE (free-to-talk). The corrected substrate earns green: thinking would show as output, so quiet means not-thinking"
        );
    }

    #[test]
    fn quiet_at_the_window_boundary_reads_idle() {
        assert_eq!(
            presence_from_quiet(BUSY_WINDOW),
            Presence::Idle,
            "the busy-window is half-open: < window is busy, >= window is the free idle"
        );
    }

    fn member(name: &str, presence: Presence) -> RosterMember {
        RosterMember {
            routing_id: name.to_string(),
            name: name.to_string(),
            identity: format!("{name}:claude-code"),
            role: RosterRole::Other,
            presence,
        }
    }

    #[test]
    fn a_working_terminal_drives_a_busy_member_no_false_idle() {
        let roster = vec![member("dev", Presence::Idle)];
        let changes = activity_presence_changes(&roster, |_| Some(Duration::from_millis(50)));
        assert_eq!(
            changes,
            vec![("dev".to_string(), Presence::Busy)],
            "a member whose terminal is actively emitting flips to BUSY — the tool-wait falsifier: a working agent never reads idle while its terminal ticks"
        );
    }

    #[test]
    fn sustained_terminal_quiet_emits_a_busy_to_idle_change() {
        let roster = vec![member("dev", Presence::Busy)];
        let changes =
            activity_presence_changes(&roster, |_| Some(BUSY_WINDOW + Duration::from_secs(2)));
        assert_eq!(
            changes,
            vec![("dev".to_string(), Presence::Idle)],
            "a member quiet past the busy-window earns the confident green IDLE — the corrected model ships real free, not a hollow guess"
        );
    }

    #[test]
    fn a_stable_state_emits_no_change() {
        let roster = vec![member("dev", Presence::Busy)];
        let changes = activity_presence_changes(&roster, |_| Some(Duration::from_millis(100)));
        assert!(
            changes.is_empty(),
            "a member already BUSY with recent terminal output must not re-emit; got {changes:?}"
        );
    }

    #[test]
    fn a_member_with_no_terminal_signal_is_left_untouched_offline_owned_by_roster_leave() {
        let roster = vec![member("dev", Presence::Online)];
        let changes = activity_presence_changes(&roster, |_| None);
        assert!(
            changes.is_empty(),
            "a registered member with no resolvable live-terminal signal is LEFT UNTOUCHED — the activity tick stays silent (never fabricates offline). Offline is owned by the roster-leave path; registered != offline. Got {changes:?}"
        );
    }

    #[test]
    fn quiet_resolves_through_session_id_to_terminal_id_never_by_role() {
        use crate::room::spawn::spawn_pty_retained;

        let dir = tempfile::TempDir::new().expect("dir");
        let routing = RoutingStore::load(dir.path().to_path_buf());
        let registry = LivePtyRegistry::new();

        let terminal_id = "ccb4298d-1111-2222-3333-444455556666";
        routing.register_agent("sess-dev", "dev", terminal_id, "2026-06-13T10:00:00Z");
        let live = spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[])
            .expect("spawn");
        registry.insert(terminal_id.to_string(), live);

        assert!(
            terminal_quiet_for_id(&routing, &registry, "sess-dev").is_some(),
            "the routing-id keys the lookup: session-id -> linked terminal-id -> the live PTY's quiet duration. The roster row is keyed by this same session-id, so presence targets the right row even when two agents share a role display-name"
        );
        assert!(
            registry.quiet_for("dev").is_none(),
            "the bug-under-the-bug: the registry is keyed by terminal-id, so a lookup by the ROLE display-name ('dev') MUST miss — keying off role is exactly what made every dot read offline"
        );
    }

    #[test]
    fn need_and_offline_states_are_never_overridden_by_activity() {
        let roster = vec![
            member("dev", Presence::Blocked),
            member("pm", Presence::Help),
            member("designer", Presence::Offline),
        ];
        let changes = activity_presence_changes(&roster, |_| Some(Duration::from_millis(0)));
        assert!(
            changes.is_empty(),
            "Blocked/Help (need) are owned by the owner-park path and Offline by the roster-leave path — the activity tick must never override them; got {changes:?}"
        );
    }
}
