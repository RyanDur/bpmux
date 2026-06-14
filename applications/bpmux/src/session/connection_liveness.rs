use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct ConnectionLiveness {
    inner: Mutex<HashMap<String, RoleConnection>>,
}

#[derive(Clone)]
struct RoleConnection {
    held_waits: u32,
    last_active: Instant,
    aborted: bool,
}

impl Default for ConnectionLiveness {
    fn default() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl ConnectionLiveness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn wait_entered(&self, role: &str) {
        self.wait_entered_at(role, Instant::now());
    }

    pub fn wait_entered_at(&self, role: &str, now: Instant) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(role.to_string()).or_insert(RoleConnection {
            held_waits: 0,
            last_active: now,
            aborted: false,
        });
        entry.held_waits += 1;
        entry.last_active = now;
        entry.aborted = false;
    }

    pub fn wait_completed(&self, role: &str) {
        self.wait_completed_at(role, Instant::now());
    }

    pub fn wait_completed_at(&self, role: &str, now: Instant) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = map.get_mut(role) {
            entry.held_waits = entry.held_waits.saturating_sub(1);
            entry.last_active = now;
        }
    }

    pub fn wait_aborted(&self, role: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = map.get_mut(role) {
            entry.held_waits = entry.held_waits.saturating_sub(1);
            if entry.held_waits == 0 {
                entry.aborted = true;
            }
        }
    }

    pub fn mark_gone(&self, role: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = map.get_mut(role) {
            entry.held_waits = 0;
            entry.aborted = true;
        }
    }

    pub fn seen(&self, role: &str) {
        self.seen_at(role, Instant::now());
    }

    pub fn seen_at(&self, role: &str, now: Instant) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(role.to_string()).or_insert(RoleConnection {
            held_waits: 0,
            last_active: now,
            aborted: false,
        });
        entry.last_active = now;
        entry.aborted = false;
    }

    pub fn tracked(&self, role: &str) -> bool {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.contains_key(role)
    }

    pub fn is_connected(&self, role: &str, grace: Duration) -> bool {
        self.is_connected_at(role, grace, Instant::now())
    }

    pub fn is_connected_at(&self, role: &str, grace: Duration, now: Instant) -> bool {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(role) {
            Some(entry) if entry.held_waits > 0 => true,
            Some(entry) if entry.aborted => false,
            Some(entry) => now.saturating_duration_since(entry.last_active) < grace,
            None => false,
        }
    }

    pub fn connected_roles(&self, grace: Duration) -> Vec<String> {
        self.connected_roles_at(grace, Instant::now())
    }

    pub fn connected_roles_at(&self, grace: Duration, now: Instant) -> Vec<String> {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.iter()
            .filter(|(_, entry)| {
                entry.held_waits > 0
                    || (!entry.aborted && now.saturating_duration_since(entry.last_active) < grace)
            })
            .map(|(role, _)| role.clone())
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const GRACE: Duration = Duration::from_secs(10);

    #[test]
    fn an_agent_holding_a_wait_is_connected() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.wait_entered_at("dev", t0);
        assert!(
            live.is_connected_at("dev", GRACE, t0),
            "an agent currently holding a /api/wait long-poll is connected — the held connection IS the liveness"
        );
    }

    #[test]
    fn a_completed_wait_keeps_the_agent_connected_within_grace_then_drops_it() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.wait_entered_at("dev", t0);
        live.wait_completed_at("dev", t0);
        assert!(
            live.is_connected_at("dev", GRACE, t0 + Duration::from_secs(3)),
            "between turns the wait completes + the agent will re-poll — within grace it stays connected (working = alive), so a busy agent never flickers out"
        );
        assert!(
            !live.is_connected_at("dev", GRACE, t0 + GRACE + Duration::from_secs(1)),
            "past grace with no re-poll and no new activity, the agent is gone — silent-death is bounded by grace, never stale-forever"
        );
    }

    #[test]
    fn a_clean_wait_abort_disconnects_instantly_not_after_grace() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.wait_entered_at("dev", t0);
        live.wait_aborted("dev");
        assert!(
            !live.is_connected_at("dev", GRACE, t0),
            "a killed-while-idle agent aborts its held wait (client-disconnect) — the drop-guard marks it gone INSTANTLY, no grace wait, because a clean close is unambiguous proof it left"
        );
    }

    #[test]
    fn a_re_entered_wait_after_an_abort_reconnects() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.wait_entered_at("dev", t0);
        live.wait_aborted("dev");
        live.wait_entered_at("dev", t0 + Duration::from_secs(1));
        assert!(
            live.is_connected_at("dev", GRACE, t0 + Duration::from_secs(1)),
            "an agent that restarts and re-establishes its wait clears the aborted flag — reconnect heals"
        );
    }

    #[test]
    fn a_concurrent_overlapping_wait_keeps_connected_until_the_last_releases() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.wait_entered_at("dev", t0);
        live.wait_entered_at("dev", t0);
        live.wait_completed_at("dev", t0);
        assert!(
            live.is_connected_at("dev", GRACE, t0),
            "two overlapping waits (a re-poll racing the prior's completion) — still held by one, still connected; the count guards against a spurious drop"
        );
    }

    #[test]
    fn connected_roles_lists_only_the_live_ones() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.wait_entered_at("dev", t0);
        live.wait_entered_at("pm", t0);
        live.wait_completed_at("pm", t0);
        live.wait_entered_at("designer", t0);
        live.wait_aborted("designer");
        let mut roles = live.connected_roles_at(GRACE, t0 + Duration::from_secs(1));
        roles.sort();
        assert_eq!(
            roles,
            vec!["dev".to_string(), "pm".to_string()],
            "connected = holding a wait (dev) or recently-active within grace (pm); designer aborted = gone. This set IS the roster membership."
        );
    }

    #[test]
    fn mark_gone_disconnects_instantly_for_a_clean_session_end() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.seen_at("dev", t0);
        live.mark_gone("dev");
        assert!(
            !live.is_connected_at("dev", GRACE, t0),
            "a clean SessionEnd explicitly marks the agent gone — instant leave, no grace wait, because a clean exit is unambiguous proof it left (the keepalive's PID-watch only covers ungraceful death, at grace latency)"
        );
        assert!(
            live.tracked("dev"),
            "still tracked so the reconcile owns the leave; a fresh register's seen() clears the gone flag and reconnects"
        );
    }

    #[test]
    fn an_unknown_role_is_not_connected() {
        let live = ConnectionLiveness::new();
        assert!(
            !live.is_connected_at("ghost", GRACE, Instant::now()),
            "a role that never connected is not in the set — membership is connectedness, nothing fabricated"
        );
    }

    #[test]
    fn seen_marks_a_just_registered_agent_connected_before_its_first_wait() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.seen_at("dev", t0);
        assert!(
            live.is_connected_at("dev", GRACE, t0 + Duration::from_secs(2)),
            "register is an authenticated call → seen() marks the agent connected during the register→first-wait window, so the reconcile never prematurely drops a just-registered agent"
        );
    }

    #[test]
    fn tracked_distinguishes_a_connection_agent_from_an_untracked_fixture() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.wait_entered_at("dev", t0);
        assert!(
            live.tracked("dev"),
            "an agent the liveness has seen is tracked — the reconcile may govern its membership"
        );
        assert!(
            !live.tracked("seeded-fixture"),
            "a roster member the liveness never saw (a test seed, or another participant-type) is UNTRACKED — the reconcile must leave it alone, never reaping what it has no connection-data for"
        );
    }

    #[test]
    fn a_tracked_agent_that_went_silent_is_tracked_but_not_connected_the_reconcile_leave_signal() {
        let live = ConnectionLiveness::new();
        let t0 = Instant::now();
        live.wait_entered_at("dev", t0);
        live.wait_completed_at("dev", t0);
        let later = t0 + GRACE + Duration::from_secs(1);
        assert!(
            live.tracked("dev") && !live.is_connected_at("dev", GRACE, later),
            "tracked + not-connected is exactly the reconcile's leave-signal: this agent WAS connected and is now gone, so its roster row should be removed"
        );
    }
}
