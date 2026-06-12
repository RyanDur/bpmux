use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

fn short(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub struct RoutingStore {
    identity: Mutex<RoutingIdentity>,
    dir: PathBuf,
}

impl RoutingStore {
    pub fn load(dir: PathBuf) -> Self {
        let identity = RoutingIdentity::load(&dir);
        Self {
            identity: Mutex::new(identity),
            dir,
        }
    }

    pub fn register(&self, session_id: &str, display: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.register(session_id, display);
        let _ = identity.save(&self.dir);
    }

    pub fn link_terminal(&self, session_id: &str, terminal_id: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.link_terminal(session_id, terminal_id);
    }

    pub fn resolve(&self, display: &str) -> Vec<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.resolve(display)
    }

    pub fn rename(&self, session_id: &str, new_display: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.rename(session_id, new_display);
        let _ = identity.save(&self.dir);
    }

    pub fn drain_floor(&self, session_id: &str, client_ack: Option<&str>) -> Option<String> {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let stored = identity.cursor(session_id);
        let floor = later_cursor(stored.as_deref(), client_ack);
        if let Some(ref position) = floor {
            identity.advance_cursor(session_id, position);
            let _ = identity.save(&self.dir);
        }
        floor
    }

    pub fn delivery_terminals(&self, to: Option<&[String]>, sender: &str) -> Vec<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let sender_ids = identity.resolve(short(sender));
        let recipient_ids: Vec<String> = match to {
            None => identity.all_ids(),
            Some(names)
                if names
                    .iter()
                    .any(|n| n == crate::session::message::BROADCAST_ALL) =>
            {
                identity.all_ids()
            }
            Some(names) => names
                .iter()
                .flat_map(|name| identity.resolve(short(name)))
                .collect(),
        };
        recipient_ids
            .into_iter()
            .filter(|id| !sender_ids.contains(id))
            .filter_map(|id| identity.terminal_for(&id))
            .collect()
    }

    pub fn register_agent(
        &self,
        session_id: &str,
        display: &str,
        terminal_id: &str,
        job_start: &str,
    ) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let fresh = identity.cursor(session_id).is_none();
        if fresh {
            identity.register(session_id, display);
            identity.advance_cursor(session_id, job_start);
        }
        identity.link_terminal(session_id, terminal_id);
        let _ = identity.save(&self.dir);
    }
}

fn later_cursor(a: Option<&str>, b: Option<&str>) -> Option<String> {
    fn parse(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    }
    match (a, b) {
        (Some(x), Some(y)) => match (parse(x), parse(y)) {
            (Some(px), Some(py)) => Some(if px >= py {
                x.to_string()
            } else {
                y.to_string()
            }),
            (Some(_), None) => Some(x.to_string()),
            (None, _) => Some(y.to_string()),
        },
        (Some(x), None) => Some(x.to_string()),
        (None, Some(y)) => Some(y.to_string()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
    fn drain_floor_clamps_a_stale_client_cursor_forward_the_durable_cure_for_tmp_replay() {
        let dir = tempfile::TempDir::new().expect("dir");
        let store = RoutingStore::load(dir.path().to_path_buf());
        store.register("sess-1", "dev");

        let first = store.drain_floor("sess-1", Some("2026-06-11T20:00:05Z"));
        assert_eq!(
            first.as_deref(),
            Some("2026-06-11T20:00:05Z"),
            "the client's ack advances the server-tracked drain position"
        );

        let stale = store.drain_floor("sess-1", Some("2026-06-11T20:00:01Z"));
        assert_eq!(
            stale.as_deref(),
            Some("2026-06-11T20:00:05Z"),
            "a STALE client cursor (an old /tmp value) is clamped FORWARD to the server's drained position — never moves the floor back, so it can never replay already-drained events (the durable cure for the stale-/tmp replay class)"
        );

        let fresher = store.drain_floor("sess-1", Some("2026-06-11T20:00:09Z"));
        assert_eq!(
            fresher.as_deref(),
            Some("2026-06-11T20:00:09Z"),
            "a fresher client ack monotonically advances the durable cursor"
        );

        let reloaded = RoutingIdentity::load(dir.path());
        assert_eq!(
            reloaded.cursor("sess-1").as_deref(),
            Some("2026-06-11T20:00:09Z"),
            "the durable cursor survives a server restart — the drain position is recovered from disk, not a volatile /tmp file"
        );
    }

    #[test]
    fn later_cursor_parses_rather_than_lexically_compares_so_subsecond_precision_is_correct() {
        assert_eq!(
            later_cursor(Some("2026-06-11T20:00:01Z"), Some("2026-06-11T20:00:01.5Z")).as_deref(),
            Some("2026-06-11T20:00:01.5Z"),
            "the later instant wins by PARSED time — a lexical string compare would wrongly pick the no-fraction form ('Z' > '.5') and lose half a second of events"
        );
    }

    #[test]
    fn routing_store_rename_updates_the_display_for_the_fixed_id_and_persists_so_the_new_name_resolves()
     {
        let dir = tempfile::TempDir::new().expect("dir");
        {
            let store = RoutingStore::load(dir.path().to_path_buf());
            store.register("sess-1", "dev");
            store.rename("sess-1", "dev-renamed");
        }
        let reloaded = RoutingIdentity::load(dir.path());
        assert_eq!(
            reloaded.resolve("dev-renamed"),
            vec!["sess-1".to_string()],
            "rename moves the DISPLAY for the fixed session-id and persists — the new name resolves to the same id, so the durable cursor stays reachable across a rename"
        );
        assert!(
            reloaded.resolve("dev").is_empty(),
            "the OLD display no longer resolves — the route-key (session-id) never moved, only the display attribute"
        );
    }

    #[test]
    fn routing_store_handle_persists_each_registration_so_a_reload_recovers_it() {
        let dir = tempfile::TempDir::new().expect("dir");
        {
            let store = RoutingStore::load(dir.path().to_path_buf());
            store.register("sess-1", "dev");
            store.link_terminal("sess-1", "term-A");
        }
        let reloaded = RoutingIdentity::load(dir.path());
        assert_eq!(
            reloaded.resolve("dev"),
            vec!["sess-1".to_string()],
            "the shared handle persists the identity on every mutation — a reload (server restart) recovers it without the caller remembering to save"
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
    fn register_agent_fresh_floors_the_cursor_at_job_start_so_gate_window_messages_drain() {
        let dir = tempfile::TempDir::new().expect("dir");
        let store = RoutingStore::load(dir.path().to_path_buf());
        store.register_agent("sess-fresh", "dev", "term-A", "2026-06-12T10:00:00Z");

        let reloaded = RoutingIdentity::load(dir.path());
        assert_eq!(
            reloaded.cursor("sess-fresh").as_deref(),
            Some("2026-06-12T10:00:00Z"),
            "a FRESH agent's cursor floors at the agent-JOB-START — not empty (would flood all history) nor the register-instant (would lose the gate-window) — so a to:[role] sent during the ~1s gate-window (after job-start, before register) still drains"
        );
        assert_eq!(
            reloaded.resolve("dev"),
            vec!["sess-fresh".to_string()],
            "register_agent registers the display=role so the agent is routable"
        );
    }

    #[test]
    fn delivery_terminals_routes_by_id_excluding_the_sender_and_offline_ids() {
        let dir = tempfile::TempDir::new().expect("dir");
        let store = RoutingStore::load(dir.path().to_path_buf());
        store.register_agent("sess-dev", "dev", "term-dev", "2026-06-12T10:00:00Z");
        store.register_agent("sess-pm", "pm", "term-pm", "2026-06-12T10:00:00Z");
        store.register("sess-offline", "designer");

        let mut broadcast = store.delivery_terminals(None, "ryan");
        broadcast.sort();
        assert_eq!(
            broadcast,
            vec!["term-dev".to_string(), "term-pm".to_string()],
            "a broadcast routes by id to every LIVE-linked terminal; an offline id (registered, no terminal) is excluded from live-delivery — it drains durably on reconnect instead, never a live write to nobody"
        );

        let mut explicit_all = store.delivery_terminals(Some(&["all".to_string()]), "ryan");
        explicit_all.sort();
        assert_eq!(
            explicit_all, broadcast,
            "an explicit BROADCAST_ALL to-list routes identically to to=None"
        );

        assert_eq!(
            store.delivery_terminals(Some(&["dev".to_string()]), "pm"),
            vec!["term-dev".to_string()],
            "a scoped to:[dev] resolves dev's DISPLAY to its id to its terminal and reaches ONLY it"
        );

        let from_dev = store.delivery_terminals(None, "dev:claude-code");
        assert_eq!(
            from_dev,
            vec!["term-pm".to_string()],
            "the SENDER's own terminal is excluded (no self-echo) and the sender is matched by short-name across a full identity; other terminals still receive"
        );
    }

    #[test]
    fn register_agent_reclaim_preserves_a_renamed_display_not_clobbered_by_the_default_name() {
        let dir = tempfile::TempDir::new().expect("dir");
        let store = RoutingStore::load(dir.path().to_path_buf());
        store.register_agent("sess-1", "bpmux-dev", "term-A", "2026-06-12T10:00:00Z");
        store.rename("sess-1", "dev");

        store.register_agent("sess-1", "bpmux-dev", "term-B", "2026-06-12T11:00:00Z");

        let reloaded = RoutingIdentity::load(dir.path());
        assert_eq!(
            reloaded.resolve("dev"),
            vec!["sess-1".to_string()],
            "a returning agent keeps the owner-forged name (dev) — re-detection re-supplies the folder-default (bpmux-dev) but must NOT clobber the rename back"
        );
        assert!(
            reloaded.resolve("bpmux-dev").is_empty(),
            "the folder-default name no longer resolves — the rename held across the reclaim, the display is sourced once at first-sight"
        );
    }

    #[test]
    fn register_agent_reclaim_preserves_the_durable_cursor_not_reset_to_the_new_job_start() {
        let dir = tempfile::TempDir::new().expect("dir");
        let store = RoutingStore::load(dir.path().to_path_buf());
        store.register_agent("sess-ret", "dev", "term-A", "2026-06-12T09:00:00Z");
        store.drain_floor("sess-ret", Some("2026-06-12T12:00:00Z"));

        store.register_agent("sess-ret", "dev", "term-B", "2026-06-12T15:00:00Z");

        let reloaded = RoutingIdentity::load(dir.path());
        assert_eq!(
            reloaded.cursor("sess-ret").as_deref(),
            Some("2026-06-12T12:00:00Z"),
            "a RETURNING agent preserves its durable drain-position (noon) — re-detecting it at a NEW job-start (15:00) must NOT reset the cursor, which would skip the offline 12:00-15:00 messages the durable-reclaim must still deliver"
        );
    }
}
