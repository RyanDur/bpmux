use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

const STORE_FILE: &str = "routing_identity.json";

#[derive(Default, Serialize, Deserialize)]
pub struct RoutingIdentity {
    display_by_id: HashMap<String, String>,
    cursor_by_id: HashMap<String, String>,
    #[serde(skip)]
    terminal_by_id: HashMap<String, String>,
}

impl RoutingIdentity {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, session_id: &str, display: &str) {
        self.display_by_id
            .insert(session_id.to_string(), display.to_string());
        self.cursor_by_id.entry(session_id.to_string()).or_default();
    }

    pub fn advance_cursor(&mut self, session_id: &str, cursor: &str) {
        if let Some(slot) = self.cursor_by_id.get_mut(session_id) {
            *slot = cursor.to_string();
        }
    }

    pub fn cursor(&self, session_id: &str) -> Option<String> {
        self.cursor_by_id.get(session_id).cloned()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let json = serde_json::to_string(self).map_err(std::io::Error::other)?;
        std::fs::write(dir.join(STORE_FILE), json)
    }

    pub fn load(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(STORE_FILE))
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    pub fn rename(&mut self, session_id: &str, new_display: &str) {
        if let Some(slot) = self.display_by_id.get_mut(session_id) {
            *slot = new_display.to_string();
        }
    }

    pub fn resolve(&self, display: &str) -> Vec<String> {
        self.display_by_id
            .iter()
            .filter(|(_, d)| d.as_str() == display)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn contains_id(&self, session_id: &str) -> bool {
        self.display_by_id.contains_key(session_id)
    }

    pub fn missed_after_cursor(&self, session_id: &str, event_cursors: &[String]) -> Vec<String> {
        let cursor = self.cursor(session_id).unwrap_or_default();
        event_cursors
            .iter()
            .filter(|candidate| **candidate > cursor)
            .cloned()
            .collect()
    }

    pub fn resolve_recipients(&self, to: Option<&[String]>) -> Vec<String> {
        match to {
            None => self.display_by_id.keys().cloned().collect(),
            Some(displays) => displays
                .iter()
                .flat_map(|display| self.resolve(display))
                .collect(),
        }
    }

    pub fn link_terminal(&mut self, session_id: &str, terminal_id: &str) {
        self.terminal_by_id
            .insert(session_id.to_string(), terminal_id.to_string());
    }

    pub fn terminal_for(&self, session_id: &str) -> Option<String> {
        self.terminal_by_id.get(session_id).cloned()
    }

    pub fn display_for(&self, session_id: &str) -> Option<String> {
        self.display_by_id.get(session_id).cloned()
    }

    pub fn session_for_terminal(&self, terminal_id: &str) -> Option<String> {
        self.terminal_by_id
            .iter()
            .find(|(_, linked)| linked.as_str() == terminal_id)
            .map(|(session_id, _)| session_id.clone())
    }

    pub fn is_terminal_linked(&self, terminal_id: &str) -> bool {
        self.terminal_by_id
            .values()
            .any(|linked| linked == terminal_id)
    }

    pub fn display_for_terminal(&self, terminal_id: &str) -> Option<String> {
        let session_id = self
            .terminal_by_id
            .iter()
            .find(|(_, linked)| linked.as_str() == terminal_id)
            .map(|(session_id, _)| session_id.clone())?;
        self.display_by_id.get(&session_id).cloned()
    }

    pub fn reap_unclaimed_duplicates_of(&mut self, display: &str) -> Vec<String> {
        let reaped: Vec<String> = self
            .display_by_id
            .iter()
            .filter(|(id, d)| {
                d.as_str() == display && !self.terminal_by_id.contains_key(id.as_str())
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in &reaped {
            self.display_by_id.remove(id);
            self.cursor_by_id.remove(id);
        }
        reaped
    }

    pub fn unclaimed(&self) -> Vec<(String, String)> {
        self.display_by_id
            .iter()
            .filter(|(id, _)| !self.terminal_by_id.contains_key(*id))
            .map(|(id, display)| (id.clone(), display.clone()))
            .collect()
    }

    pub fn all_ids(&self) -> Vec<String> {
        self.display_by_id.keys().cloned().collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn routing_survives_rename_session_id_is_the_key_display_is_a_mutable_attribute() {
        let mut identity = RoutingIdentity::new();
        identity.register("sess-uuid-1", "dev");
        assert_eq!(identity.resolve("dev"), vec!["sess-uuid-1".to_string()]);

        identity.rename("sess-uuid-1", "dev-renamed");
        assert_eq!(
            identity.resolve("dev-renamed"),
            vec!["sess-uuid-1".to_string()],
            "rename moves the DISPLAY; the new display must still resolve to the same session-id"
        );
        assert!(
            identity.resolve("dev").is_empty(),
            "the OLD display must no longer route — the route-key (session-id) never moved"
        );
        assert!(identity.contains_id("sess-uuid-1"));
    }

    #[test]
    fn reconnecting_same_session_id_reclaims_its_cursor_does_not_reset_it() {
        let mut identity = RoutingIdentity::new();
        identity.register("sess-1", "dev");
        identity.advance_cursor("sess-1", "2026-06-11T20:00:00Z");

        identity.register("sess-1", "dev-renamed");

        assert_eq!(
            identity.cursor("sess-1"),
            Some("2026-06-11T20:00:00Z".to_string()),
            "a reconnecting same-session-id reclaims its existing cursor (drains from where it left off) — register preserves the cursor while it updates the display"
        );
    }

    #[test]
    fn the_store_persists_to_the_dir_and_a_reload_recovers_id_display_and_cursor() {
        let dir = tempfile::TempDir::new().expect("dir");
        {
            let mut identity = RoutingIdentity::new();
            identity.register("sess-1", "dev");
            identity.advance_cursor("sess-1", "2026-06-11T20:00:00Z");
            identity.save(dir.path()).expect("save");
        }
        let reloaded = RoutingIdentity::load(dir.path());
        assert_eq!(
            reloaded.resolve("dev"),
            vec!["sess-1".to_string()],
            "a reload after restart recovers the id->display mapping (dir-persisted identity)"
        );
        assert_eq!(
            reloaded.cursor("sess-1"),
            Some("2026-06-11T20:00:00Z".to_string()),
            "a reload recovers the per-id cursor — durable across restart (the offline->reconnect reclaim base)"
        );
    }

    #[test]
    fn drain_identification_returns_the_missed_set_strictly_after_the_reclaimed_cursor() {
        let mut identity = RoutingIdentity::new();
        identity.register("sess-1", "dev");
        identity.advance_cursor("sess-1", "2026-06-11T20:00:02Z");
        let events = vec![
            "2026-06-11T20:00:01Z".to_string(),
            "2026-06-11T20:00:02Z".to_string(),
            "2026-06-11T20:00:03Z".to_string(),
            "2026-06-11T20:00:04Z".to_string(),
        ];
        assert_eq!(
            identity.missed_after_cursor("sess-1", &events),
            vec![
                "2026-06-11T20:00:03Z".to_string(),
                "2026-06-11T20:00:04Z".to_string()
            ],
            "the reclaimed cursor identifies the missed-events SET — strictly AFTER the cursor (the cursor position was the last delivered, not missed)"
        );
    }

    #[test]
    fn linking_a_terminal_lets_delivery_find_the_live_pty_and_relinks_on_reconnect() {
        let mut identity = RoutingIdentity::new();
        identity.register("sess-1", "dev");
        assert_eq!(
            identity.terminal_for("sess-1"),
            None,
            "a registered id with no live terminal yet (offline, or an orphaned id whose owner was deleted) links to NO pty — delivery to it routes to nobody, never mis-routes"
        );

        identity.link_terminal("sess-1", "term-A");
        assert_eq!(identity.terminal_for("sess-1"), Some("term-A".to_string()));

        identity.link_terminal("sess-1", "term-B");
        assert_eq!(
            identity.terminal_for("sess-1"),
            Some("term-B".to_string()),
            "a reconnecting same-session-id re-links to its NEW live pty — delivery follows the agent across a restart"
        );
    }

    #[test]
    fn unclaimed_is_the_registered_ids_with_no_live_terminal_the_reclaim_dropdown_set() {
        let mut identity = RoutingIdentity::new();
        identity.register("sess-live", "dev");
        identity.register("sess-orphan", "former-pm");
        identity.link_terminal("sess-live", "term-A");

        assert_eq!(
            identity.unclaimed(),
            vec![("sess-orphan".to_string(), "former-pm".to_string())],
            "unclaimed = a registered id whose owner is gone (no live terminal) — the set a user picks from to reclaim a slice of the conversation; a claimed (live-linked) id is NOT offered"
        );

        identity.link_terminal("sess-orphan", "term-B");
        assert!(
            identity.unclaimed().is_empty(),
            "claiming the orphan (linking a terminal) removes it from the unclaimed set — nothing unclaimed → no dropdown"
        );
    }

    #[test]
    fn resolve_recipients_broadcast_is_all_ids_scoped_is_resolved_ids_unknown_is_no_leak() {
        let mut identity = RoutingIdentity::new();
        identity.register("sess-1", "dev");
        identity.register("sess-2", "pm");

        let mut broadcast = identity.resolve_recipients(None);
        broadcast.sort();
        assert_eq!(
            broadcast,
            vec!["sess-1".to_string(), "sess-2".to_string()],
            "broadcast (to=None) routes to every registered instance"
        );

        assert_eq!(
            identity.resolve_recipients(Some(&["pm".to_string()])),
            vec!["sess-2".to_string()],
            "a scoped message resolves the display-name to its session-id"
        );

        assert!(
            identity
                .resolve_recipients(Some(&["ghost".to_string()]))
                .is_empty(),
            "a scoped message to an unknown display routes to NO instance — no mis-route/leak"
        );
    }

    #[test]
    fn display_for_terminal_reverse_resolves_the_agent_bound_to_a_terminal() {
        let mut identity = RoutingIdentity::new();
        identity.register("sess-dev", "dev");
        identity.link_terminal("sess-dev", "term-1");
        assert_eq!(
            identity.display_for_terminal("term-1"),
            Some("dev".to_string()),
            "the reverse lookup finds the DISPLAY bound to a terminal — used on terminal-death to roster-leave the right agent so the roster tracks terminal liveness"
        );
        assert_eq!(
            identity.display_for_terminal("term-unbound"),
            None,
            "a terminal bound to no session resolves to no agent — death of an unregistered terminal leaves the roster untouched"
        );
    }

    #[test]
    fn reap_removes_unclaimed_duplicates_of_a_live_display_but_never_an_offline_other_display() {
        let mut identity = RoutingIdentity::new();
        identity.register("dev-live", "dev");
        identity.link_terminal("dev-live", "term-dev");
        identity.register("dev-stale-a", "dev");
        identity.advance_cursor("dev-stale-a", "2026-06-13T00:00:00Z");
        identity.register("dev-stale-b", "dev");
        identity.register("pm-offline", "pm");
        identity.advance_cursor("pm-offline", "2026-06-13T00:00:00Z");

        let reaped = identity.reap_unclaimed_duplicates_of("dev");

        assert!(
            reaped.contains(&"dev-stale-a".to_string())
                && reaped.contains(&"dev-stale-b".to_string()),
            "the unclaimed (no live terminal) dev duplicates — the splinters from the old per-cwd model — are reaped; got {reaped:?}"
        );
        assert_eq!(
            identity.resolve("dev"),
            vec!["dev-live".to_string()],
            "the LIVE (terminal-bound) dev is preserved — only the one true dev remains"
        );
        assert!(
            identity.cursor("dev-stale-a").is_none(),
            "a reaped id's durable cursor is removed too (it was dead history)"
        );
        assert_eq!(
            identity.resolve("pm"),
            vec!["pm-offline".to_string()],
            "an OFFLINE agent of a DIFFERENT display is untouched — reap is scoped to the display whose live terminal just bound, so a legitimately-offline agent (no live sibling) keeps its id"
        );
        assert_eq!(
            identity.cursor("pm-offline"),
            Some("2026-06-13T00:00:00Z".to_string()),
            "the offline agent's durable cursor survives — the discriminator never breaks offline-reconnect"
        );
    }
}
