use std::path::PathBuf;
use std::sync::Mutex;

use super::identity::RoutingIdentity;

fn short(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub fn bridge_agent_name(terminal_id: &str) -> String {
    let prefix = &terminal_id[..terminal_id.len().min(8)];
    format!("agent-{prefix}")
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

    pub fn cursor(&self, session_id: &str) -> Option<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.cursor(session_id)
    }

    pub fn mark_delivered(&self, session_id: &str, ts: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let current = identity.cursor(session_id);
        if let Some(later) = later_cursor(current.as_deref(), Some(ts))
            && current.as_deref() != Some(later.as_str())
        {
            identity.advance_cursor(session_id, &later);
            let _ = identity.save(&self.dir);
        }
    }

    pub fn delivery_targets(&self, to: Option<&[String]>, sender: &str) -> Vec<(String, String)> {
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
            .filter_map(|id| identity.terminal_for(&id).map(|terminal| (id, terminal)))
            .filter(|(_, terminal)| bridge_agent_name(terminal) != short(sender))
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
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

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
    fn delivery_targets_routes_by_id_excluding_the_sender_and_offline_ids() {
        let dir = tempfile::TempDir::new().expect("dir");
        let store = RoutingStore::load(dir.path().to_path_buf());
        store.register_agent("sess-dev", "dev", "term-dev", "2026-06-12T10:00:00Z");
        store.register_agent("sess-pm", "pm", "term-pm", "2026-06-12T10:00:00Z");
        store.register("sess-offline", "designer");

        let terminals = |targets: Vec<(String, String)>| {
            let mut t: Vec<String> = targets.into_iter().map(|(_, terminal)| terminal).collect();
            t.sort();
            t
        };

        let broadcast = terminals(store.delivery_targets(None, "ryan"));
        assert_eq!(
            broadcast,
            vec!["term-dev".to_string(), "term-pm".to_string()],
            "a broadcast routes by id to every LIVE-linked terminal; an offline id (registered, no terminal) is excluded from live-delivery — it drains durably on reconnect instead, never a live write to nobody"
        );

        assert_eq!(
            terminals(store.delivery_targets(Some(&["all".to_string()]), "ryan")),
            broadcast,
            "an explicit BROADCAST_ALL to-list routes identically to to=None"
        );

        assert_eq!(
            store.delivery_targets(Some(&["dev".to_string()]), "pm"),
            vec![("sess-dev".to_string(), "term-dev".to_string())],
            "a scoped to:[dev] resolves dev's DISPLAY to its id to its terminal and reaches ONLY it — carrying the session-id so live-delivery advances dev's cursor"
        );

        assert_eq!(
            store.delivery_targets(None, "dev:claude-code"),
            vec![("sess-pm".to_string(), "term-pm".to_string())],
            "the SENDER's own terminal is excluded (no self-echo) and the sender is matched by short-name across a full identity; other terminals still receive"
        );
    }

    #[test]
    fn delivery_targets_excludes_the_senders_own_terminal_even_when_the_send_name_is_the_unresolved_bridge_name()
     {
        let dir = tempfile::TempDir::new().expect("dir");
        let store = RoutingStore::load(dir.path().to_path_buf());
        let dev_terminal = "ccb4298d-1111-2222-3333-444455556666";
        store.register_agent("sess-dev", "dev", dev_terminal, "2026-06-12T10:00:00Z");
        store.register_agent("sess-pm", "pm", "term-pm", "2026-06-12T10:00:00Z");

        let sender = bridge_agent_name(dev_terminal);
        let targets = store.delivery_targets(None, &sender);

        assert!(
            targets.iter().any(|(_, t)| t == "term-pm"),
            "non-vacuous: a broadcast still reaches a DIFFERENT agent's terminal; got {targets:?}"
        );
        assert!(
            targets.iter().all(|(_, t)| t != dev_terminal),
            "a broadcast sent under the bridge name agent-{{terminal[..8]}} must NOT route back into the sender's OWN terminal. The bridge auto-assigns this name (rest/room.rs) separately from the dir-session display ('dev'), so resolve-by-display cannot exclude it — the deterministic wiring link agent-{{terminal[..8]}} must. This is Ryan's reject: a terminal's own messages reflected back into its input; got {targets:?}"
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
