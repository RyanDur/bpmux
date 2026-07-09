#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use super::cursor_time::later_cursor;
use super::*;
use crate::storage::sqlite_test_storage;

type MirrorStore = Arc<crate::kernel::Store<crate::store::AppState, crate::store::AppAction>>;

fn wire_slice(store: &RoutingStore) -> MirrorStore {
    let mirror = crate::kernel::Store::new(
        crate::store::AppState::default(),
        Box::new(crate::store::app_reduce),
    );
    let for_dispatch = mirror.clone();
    store.bind_dispatch(crate::store::Dispatch::new(Arc::new(move |action| {
        for_dispatch.dispatch(action)
    })));
    mirror
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

fn sorted_targets(mut targets: Vec<(String, String)>) -> Vec<(String, String)> {
    targets.sort();
    targets
}

fn instant(rfc3339: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .unwrap()
        .timestamp_millis()
}

#[test]
fn the_slice_mirrors_the_routing_store_across_register_link_rename_cursor_and_reap() {
    let dir = tempfile::TempDir::new().expect("dir");
    let store = RoutingStore::new(sqlite_test_storage(dir.path()));
    let mirror = wire_slice(&store);

    store.register("sess-dev", "dev");
    store.register("sess-pm", "pm");
    store.link_terminal("sess-dev", "term-dev");
    store.link_terminal("sess-pm", "term-pm");
    store.rename("sess-pm", "pm-renamed");
    store.mark_delivered("sess-dev", "2026-06-13T10:00:00Z");
    store.mark_delivered("sess-dev", "2026-06-13T09:00:00Z");
    store.register("sess-stale", "dev");
    store.reap_unclaimed_duplicates_of("dev");

    for session in ["sess-dev", "sess-pm", "sess-stale"] {
        assert_eq!(
            mirror.select(|s| crate::store::routing::cursor(&s.routing, session)),
            store.cursor(session),
            "cursor mirror for {session}"
        );
        assert_eq!(
            mirror.select(|s| crate::store::routing::terminal_for(&s.routing, session)),
            store.terminal_for(session),
            "terminal_for mirror for {session}"
        );
        assert_eq!(
            mirror.select(|s| crate::store::routing::display_for(&s.routing, session)),
            store.display_for(session),
            "display_for mirror for {session}"
        );
    }
    for display in ["dev", "pm", "pm-renamed"] {
        assert_eq!(
            sorted(mirror.select(|s| crate::store::routing::resolve(&s.routing, display))),
            sorted(store.resolve(display)),
            "resolve mirror for {display}"
        );
    }
    for terminal in ["term-dev", "term-pm", "term-absent"] {
        assert_eq!(
            mirror.select(|s| crate::store::routing::session_for_terminal(&s.routing, terminal)),
            store.session_for_terminal(terminal),
            "session_for_terminal mirror for {terminal}"
        );
        assert_eq!(
            mirror.select(|s| crate::store::routing::is_terminal_linked(&s.routing, terminal)),
            store.is_terminal_linked(terminal),
            "is_terminal_linked mirror for {terminal}"
        );
    }
    assert_eq!(
        sorted_targets(
            mirror.select(|s| crate::store::routing::delivery_targets(&s.routing, None, "owner"))
        ),
        sorted_targets(store.delivery_targets(None, "owner")),
        "delivery_targets mirror for a broadcast — the slice and RoutingStore route identically"
    );
    assert_eq!(
        mirror.select(|s| crate::store::routing::delivery_targets(
            &s.routing,
            Some(&["dev".to_string()]),
            "pm"
        )),
        store.delivery_targets(Some(&["dev".to_string()]), "pm"),
        "delivery_targets mirror for a scoped to:[dev]"
    );
}

#[test]
fn the_slice_mirrors_register_agent_including_the_backward_cursor_reset_and_the_reap() {
    let dir = tempfile::TempDir::new().expect("dir");
    let store = RoutingStore::new(sqlite_test_storage(dir.path()));
    let mirror = wire_slice(&store);

    store.register_agent("sess-dev", "dev", "term-dev", "2026-06-12T10:00:00Z");
    store.register("sess-stale", "designer");
    store.register("sess-designer", "designer");
    store.mark_delivered("sess-designer", "2099-01-01T00:00:00Z");
    store.register_agent(
        "sess-designer",
        "designer",
        "term-designer",
        "2026-06-13T12:00:00Z",
    );
    store.mark_delivered("sess-dev", "2026-06-13T10:05:00Z");
    store.register_agent("sess-dev", "dev", "term-dev-2", "2026-06-13T11:00:00Z");

    assert_eq!(
        store.cursor("sess-designer").as_deref(),
        Some("2026-06-13T12:00:00Z"),
        "precondition: register_agent reset the poisoned 2099 cursor BACKWARD to job-start — the case a monotonic CursorAdvanced could not mirror"
    );
    assert_eq!(
        store.cursor("sess-dev").as_deref(),
        Some("2026-06-13T10:05:00Z"),
        "precondition: the returning dev whose cursor is PAST the new job-start keeps it — register_agent does not anchor forward"
    );

    for session in ["sess-dev", "sess-designer", "sess-stale"] {
        assert_eq!(
            mirror.select(|s| crate::store::routing::cursor(&s.routing, session)),
            store.cursor(session),
            "cursor mirror for {session} — the CursorAnchored decomposition tracks the backward reset AND the forward-preserve"
        );
        assert_eq!(
            mirror.select(|s| crate::store::routing::terminal_for(&s.routing, session)),
            store.terminal_for(session),
            "terminal_for mirror for {session}"
        );
    }
    for display in ["dev", "designer"] {
        assert_eq!(
            sorted(mirror.select(|s| crate::store::routing::resolve(&s.routing, display))),
            sorted(store.resolve(display)),
            "resolve mirror for {display} — register_agent's Reaped decomposition reaped the unclaimed duplicate in the slice too"
        );
    }
    assert_eq!(
        sorted_targets(
            mirror.select(|s| crate::store::routing::delivery_targets(&s.routing, None, "owner"))
        ),
        sorted_targets(store.delivery_targets(None, "owner")),
        "delivery_targets mirror after the composite register_agent sequence"
    );
}

#[test]
fn a_restart_hydrates_the_slice_identical_to_a_reloaded_routing_store_no_terminal_links() {
    let dir = tempfile::TempDir::new().expect("dir");
    let storage = sqlite_test_storage(dir.path());
    let store = RoutingStore::new(storage.clone());
    store.register("sess-dev", "dev");
    store.link_terminal("sess-dev", "term-dev");
    store.mark_delivered("sess-dev", "2026-06-13T10:00:00Z");
    store.register("sess-pm", "pm");

    let reloaded = RoutingStore::new(storage.clone());
    let hydrated = crate::kernel::Store::new(
        crate::store::AppState::default().with_routing(crate::store::routing::from_users(
            &storage.load_users().expect("load_users"),
        )),
        Box::new(crate::store::app_reduce),
    );

    for display in ["dev", "pm"] {
        assert_eq!(
            sorted(hydrated.select(|s| crate::store::routing::resolve(&s.routing, display))),
            sorted(reloaded.resolve(display)),
            "a RESTART repopulates the slice's display index identical to a reloaded RoutingStore for {display} — post-restart reads resolve the hydrated sessions"
        );
    }
    assert_eq!(
        hydrated.select(|s| crate::store::routing::cursor(&s.routing, "sess-dev")),
        reloaded.cursor("sess-dev"),
        "the delivery cursor hydrates from last_delivered_at byte-identical to the reloaded store — both boot from the same load_users, so post-restart delivery does not re-dump already-delivered messages"
    );
    assert_eq!(
        hydrated.select(|s| crate::store::routing::terminal_for(&s.routing, "sess-dev")),
        None,
        "terminal links are runtime-only — not persisted; a fresh boot hydrates identity+cursor but no terminal binding, so a reconnecting agent re-links at runtime"
    );
}

#[test]
fn short_name_strips_tool_suffix() {
    assert_eq!(short_name("dev:claude-code"), "dev");
    assert_eq!(short_name("owner:human"), "owner");
    assert_eq!(short_name("bpmux"), "bpmux");
}

#[test]
fn drain_floor_clamps_a_stale_client_cursor_forward_the_durable_cure_for_tmp_replay() {
    let dir = tempfile::TempDir::new().expect("dir");
    let storage = sqlite_test_storage(dir.path());
    let store = RoutingStore::new(storage.clone());
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

    let reloaded = RoutingStore::new(storage.clone());
    assert_eq!(
        reloaded.cursor("sess-1").as_deref().map(instant),
        Some(instant("2026-06-11T20:00:09Z")),
        "the durable cursor survives a server restart — the drain position is recovered from the users table as an instant, not a volatile /tmp file"
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
    let storage = sqlite_test_storage(dir.path());
    {
        let store = RoutingStore::new(storage.clone());
        store.register("sess-1", "dev");
        store.rename("sess-1", "dev-renamed");
    }
    let reloaded = RoutingStore::new(storage.clone());
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
    let storage = sqlite_test_storage(dir.path());
    {
        let store = RoutingStore::new(storage.clone());
        store.register("sess-1", "dev");
        store.link_terminal("sess-1", "term-A");
    }
    let reloaded = RoutingStore::new(storage.clone());
    assert_eq!(
        reloaded.resolve("dev"),
        vec!["sess-1".to_string()],
        "the shared handle persists the identity on every mutation — a reload (server restart) recovers it without the caller remembering to save"
    );
}

#[test]
fn register_agent_fresh_floors_the_cursor_at_job_start_so_gate_window_messages_drain() {
    let dir = tempfile::TempDir::new().expect("dir");
    let storage = sqlite_test_storage(dir.path());
    let store = RoutingStore::new(storage.clone());
    store.register_agent("sess-fresh", "dev", "term-A", "2026-06-12T10:00:00Z");

    let reloaded = RoutingStore::new(storage.clone());
    assert_eq!(
        reloaded.cursor("sess-fresh").as_deref().map(instant),
        Some(instant("2026-06-12T10:00:00Z")),
        "a FRESH agent's cursor floors at the agent-JOB-START — not empty (would flood all history) nor the register-instant (would lose the gate-window) — so a to:[role] sent during the ~1s gate-window (after job-start, before register) still drains"
    );
    assert_eq!(
        reloaded.resolve("dev"),
        vec!["sess-fresh".to_string()],
        "register_agent registers the display=role so the agent is routable"
    );
}

#[test]
fn register_agent_resets_a_future_cursor_to_job_start_so_a_poisoned_floor_self_heals_on_rebind() {
    let dir = tempfile::TempDir::new().expect("dir");
    let store = RoutingStore::new(sqlite_test_storage(dir.path()));
    store.register("sess-designer", "designer");
    store.mark_delivered("sess-designer", "2099-01-01T00:00:00Z");
    assert_eq!(
        store.cursor("sess-designer").as_deref(),
        Some("2099-01-01T00:00:00Z"),
        "precondition: the session carries a poisoned far-future delivery cursor (the production 2099 incident that muted the designer — every real message fails the msg.ts > cursor delivery gate)"
    );

    store.register_agent(
        "sess-designer",
        "designer",
        "term-designer",
        "2026-06-13T12:00:00Z",
    );

    assert_eq!(
        store.cursor("sess-designer").as_deref(),
        Some("2026-06-13T12:00:00Z"),
        "a stored cursor LATER than this bind's job-start is impossible — no message can have been delivered after the very job that is binding now started — so it is corruption, and register_agent resets the floor to job-start, healing the muted session so delivery resumes on the agent's next restart"
    );
}

#[test]
fn register_agent_preserves_a_past_cursor_on_reconnect_keeping_delivery_forward_only() {
    let dir = tempfile::TempDir::new().expect("dir");
    let store = RoutingStore::new(sqlite_test_storage(dir.path()));
    store.register_agent("sess-dev", "dev", "term-a", "2026-06-13T10:00:00Z");
    store.mark_delivered("sess-dev", "2026-06-13T10:05:00Z");

    store.register_agent("sess-dev", "dev", "term-b", "2026-06-13T11:00:00Z");

    assert_eq!(
        store.cursor("sess-dev").as_deref(),
        Some("2026-06-13T10:05:00Z"),
        "a reconnecting agent whose cursor is in the PAST (messages delivered before this job started) keeps its cursor — it drains forward from where it left off, NOT reset to the new job-start — so the self-heal never re-drains ambient history on a normal reconnect (the forward-only invariant the offline-broadcast test pins)"
    );
}

#[test]
fn drain_floor_rejects_a_future_client_ack_so_a_bad_ack_cannot_poison_the_floor() {
    let dir = tempfile::TempDir::new().expect("dir");
    let store = RoutingStore::new(sqlite_test_storage(dir.path()));
    store.register("sess-1", "dev");
    store.mark_delivered("sess-1", "2026-06-13T10:00:00Z");

    let floor = store.drain_floor("sess-1", Some("2099-01-01T00:00:00Z"));

    assert_eq!(
        floor.as_deref(),
        Some("2026-06-13T10:00:00Z"),
        "a client ack dated in the future is untrusted and rejected — the drain floor stays at the real stored cursor, never clamped forward into the future (the most plausible vector that muted the designer: a wait/drain call carrying a future after-cursor)"
    );
    assert_eq!(
        store.cursor("sess-1").as_deref(),
        Some("2026-06-13T10:00:00Z"),
        "the durable cursor is never advanced by a future ack — the poison cannot be persisted"
    );
}

#[test]
fn delivery_targets_routes_by_id_excluding_the_sender_and_offline_ids() {
    let dir = tempfile::TempDir::new().expect("dir");
    let store = RoutingStore::new(sqlite_test_storage(dir.path()));
    store.register_agent("sess-dev", "dev", "term-dev", "2026-06-12T10:00:00Z");
    store.register_agent("sess-pm", "pm", "term-pm", "2026-06-12T10:00:00Z");
    store.register("sess-offline", "designer");

    let terminals = |targets: Vec<(String, String)>| {
        let mut t: Vec<String> = targets.into_iter().map(|(_, terminal)| terminal).collect();
        t.sort();
        t
    };

    let broadcast = terminals(store.delivery_targets(None, "owner"));
    assert_eq!(
        broadcast,
        vec!["term-dev".to_string(), "term-pm".to_string()],
        "a broadcast routes by id to every LIVE-linked terminal; an offline id (registered, no terminal) is excluded from live-delivery — it drains durably on reconnect instead, never a live write to nobody"
    );

    assert_eq!(
        terminals(store.delivery_targets(Some(&["all".to_string()]), "owner")),
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
    let store = RoutingStore::new(sqlite_test_storage(dir.path()));
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
        "a broadcast sent under the bridge name agent-{{terminal[..8]}} must NOT route back into the sender's OWN terminal. The bridge auto-assigns this name (rest/room.rs) separately from the dir-session display ('dev'), so resolve-by-display cannot exclude it — the deterministic wiring link agent-{{terminal[..8]}} must. This is owner's reject: a terminal's own messages reflected back into its input; got {targets:?}"
    );
}

#[test]
fn register_agent_reclaim_preserves_a_renamed_display_not_clobbered_by_the_default_name() {
    let dir = tempfile::TempDir::new().expect("dir");
    let storage = sqlite_test_storage(dir.path());
    let store = RoutingStore::new(storage.clone());
    store.register_agent("sess-1", "bpmux-dev", "term-A", "2026-06-12T10:00:00Z");
    store.rename("sess-1", "dev");

    store.register_agent("sess-1", "bpmux-dev", "term-B", "2026-06-12T11:00:00Z");

    let reloaded = RoutingStore::new(storage.clone());
    assert_eq!(
        reloaded.resolve("dev"),
        vec!["sess-1".to_string()],
        "a returning agent keeps the owner-forged name (dev) — re-detection re-supplies the folder-default (bpmux-dev) but must NOT clobber the rename back, in the durable users table"
    );
    assert!(
        reloaded.resolve("bpmux-dev").is_empty(),
        "the folder-default name no longer resolves — the rename held across the reclaim, the display is sourced once at first-sight"
    );
}

#[test]
fn register_agent_reclaim_preserves_the_durable_cursor_not_reset_to_the_new_job_start() {
    let dir = tempfile::TempDir::new().expect("dir");
    let storage = sqlite_test_storage(dir.path());
    let store = RoutingStore::new(storage.clone());
    store.register_agent("sess-ret", "dev", "term-A", "2026-06-12T09:00:00Z");
    store.drain_floor("sess-ret", Some("2026-06-12T12:00:00Z"));

    store.register_agent("sess-ret", "dev", "term-B", "2026-06-12T15:00:00Z");

    let reloaded = RoutingStore::new(storage.clone());
    assert_eq!(
        reloaded.cursor("sess-ret").as_deref().map(instant),
        Some(instant("2026-06-12T12:00:00Z")),
        "a RETURNING agent preserves its durable drain-position (noon) — re-detecting it at a NEW job-start (15:00) must NOT reset the cursor, which would skip the offline 12:00-15:00 messages the durable-reclaim must still deliver"
    );
}

#[test]
fn reaping_stale_duplicates_persists_so_a_reload_recovers_only_the_live_id() {
    let dir = tempfile::TempDir::new().expect("dir");
    let storage = sqlite_test_storage(dir.path());
    let store = RoutingStore::new(storage.clone());
    store.register("dev-stale", "dev");
    store.register_agent("dev-live", "dev", "term-live", "2026-06-13T10:00:00Z");

    let reloaded = RoutingStore::new(storage.clone());
    assert_eq!(
        reloaded.resolve("dev"),
        vec!["dev-live".to_string()],
        "register_agent reaped the unclaimed stale 'dev' when the live 'dev' bound its terminal, and the reap PERSISTED — a reload recovers only the one live id, not the splinter"
    );
}

#[test]
fn register_and_drain_write_through_to_the_users_table() {
    let dir = tempfile::TempDir::new().expect("dir");
    let storage = sqlite_test_storage(dir.path());
    let store = RoutingStore::new(storage.clone());
    store.register("sess-dev", "dev:claude-code");
    store.drain_floor("sess-dev", Some("2026-06-11T20:00:05Z"));

    let users = storage.load_users().expect("load users");
    let user = users
        .iter()
        .find(|u| u.session_id == "sess-dev")
        .expect("the registered identity is a real row in the users table");
    assert_eq!(
        user.name, "dev:claude-code",
        "the opaque display persists as the user name"
    );
    assert_eq!(user.role, "dev", "the role token is short(display)");
    assert_eq!(
        user.last_delivered_at,
        Some(instant("2026-06-11T20:00:05Z")),
        "the drain-floor advance writes through into the users.last_delivered_at column as an instant"
    );
}

#[test]
fn register_agent_reap_deletes_the_unclaimed_duplicate_row_from_the_users_table() {
    let dir = tempfile::TempDir::new().expect("dir");
    let storage = sqlite_test_storage(dir.path());
    let store = RoutingStore::new(storage.clone());
    store.register("dev-stale", "dev");
    store.register_agent("dev-live", "dev", "term-live", "2026-06-13T10:00:00Z");

    let ids: Vec<String> = storage
        .load_users()
        .expect("load users")
        .into_iter()
        .map(|u| u.session_id)
        .collect();
    assert!(
        ids.contains(&"dev-live".to_string()),
        "the live claimed identity persists in the table"
    );
    assert!(
        !ids.contains(&"dev-stale".to_string()),
        "the reap of the unclaimed duplicate deletes its row from the users table"
    );
}
