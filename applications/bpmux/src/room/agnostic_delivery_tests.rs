#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::agnostic_delivery::*;
use super::wake_delivery::wake_committed_terminals;
use crate::room::{CommitmentStore, LivePtyRegistry, spawn_pty_retained};
use crate::session::MessageType;
use crate::session::message::{CiSnapshotWire, Message, StateDeltaPayload};
use crate::session::routing_identity::RoutingStore;
use std::sync::Arc;
use tokio::sync::broadcast;

fn ci_event(state: crate::ci::CiState, ts: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        ts: chrono::DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&chrono::Utc),
        message_type: MessageType::StateDelta,
        state_delta: Some(StateDeltaPayload::Ci {
            snapshot: CiSnapshotWire {
                state,
                run_id: Some(42),
                head_sha: Some("abc123".to_string()),
                display_title: Some("CI on main".to_string()),
                last_successful_poll_at: None,
                url: None,
            },
        }),
        ..Message::default()
    }
}

fn chat(from: &str, text: &str, to: Option<Vec<String>>) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        to,
        ..Message::default()
    }
}

fn chat_at(from: &str, text: &str, to: Option<Vec<String>>, ts: &str) -> Message {
    Message {
        ts: chrono::DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&chrono::Utc),
        ..chat(from, text, to)
    }
}

async fn drain_for(rx: &mut broadcast::Receiver<Vec<u8>>, needle: &str) -> String {
    let mut out = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(4);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(bytes)) if bytes.is_empty() => break,
            Ok(Ok(bytes)) => {
                out.push_str(&String::from_utf8_lossy(&bytes));
                if out.contains(needle) {
                    break;
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    out
}

fn register_live_cat(
    registry: &LivePtyRegistry,
    routing: &RoutingStore,
    session_id: &str,
    display: &str,
    terminal_id: &str,
) -> broadcast::Receiver<Vec<u8>> {
    let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
    let rx = live.subscribe();
    registry.insert(terminal_id.to_string(), live);
    routing.register_agent(session_id, display, terminal_id, "2026-06-12T10:00:00Z");
    rx
}

#[tokio::test]
async fn a_broadcast_chat_is_framed_and_written_into_a_live_agents_terminal() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let mut rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

    deliver_to_terminals(&chat("pm", "ship it", None), &registry, &routing);

    let seen = drain_for(&mut rx, "ship it").await;
    assert!(
        seen.contains("⟦bpmux⟧ pm broadcast▸") && seen.contains("ship it"),
        "a broadcast chat is delivered INTO the agent's terminal as a forgery-framed line (the agnostic receive — no agent-specific glue); got {seen:?}"
    );
}

#[tokio::test]
async fn an_inject_is_held_not_submitted_while_the_owner_is_mid_typing_into_the_agent() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let _rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

    registry.note_human_keystroke("term-dev");

    let held = deliver_to_terminals(&chat("pm", "ship it", None), &registry, &routing);

    assert_eq!(
        held.len(),
        1,
        "while the owner is mid-typing INTO the agent, the inject is HELD not submitted — clause (d) submit-isolation: the \\r must not merge his partial line + the paste-block into one Claude turn (the owner-walk submit-steal)"
    );
    assert_eq!(held[0].terminal_id, "term-dev");
    assert!(
        held[0].line.contains("ship it"),
        "the WHOLE inject is held, not just the \\r — holding only the \\r still merges the paste-block into the input-box (a polluted draft)"
    );
}

#[tokio::test]
async fn a_held_inject_delivers_once_the_owner_pauses_typing() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = Arc::new(RoutingStore::load(dir.path().to_path_buf()));
    let registry = Arc::new(LivePtyRegistry::new());
    let mut rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

    registry.note_human_keystroke("term-dev");
    let held = deliver_to_terminals(&chat("pm", "ship it", None), &registry, &routing);
    assert_eq!(held.len(), 1, "held while the owner types");

    for inject in held {
        spawn_deferred_inject(registry.clone(), routing.clone(), inject);
    }

    let seen = drain_for(&mut rx, "ship it").await;
    assert!(
        seen.contains("ship it"),
        "once the owner pauses past the debounce, the held inject delivers as its OWN turn — bounded hold, never a park; got {seen:?}"
    );
}

#[tokio::test]
async fn a_broadcast_to_an_idle_agent_submits_immediately_despite_a_server_cd_write() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let mut rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

    registry.write_to("term-dev", b"cd /work\n");

    let held = deliver_to_terminals(&chat("pm", "ship it", None), &registry, &routing);

    assert!(
        held.is_empty(),
        "a SERVER-driven write (osc7 cwd-sync / provisioning / agent_detect 'cd ...') is NOT owner-typing — a broadcast to an idle agent must submit IMMEDIATELY, never held; stamping server-writes as activity parked the first post-restart broadcast (the never-park fix-walk reject)"
    );
    let seen = drain_for(&mut rx, "ship it").await;
    assert!(
        seen.contains("ship it"),
        "the broadcast delivered immediately into the idle agent — never-park preserved; got {seen:?}"
    );
}

#[tokio::test]
async fn the_carriage_return_submits_the_line_so_a_bare_read_loop_processes_it_not_parks() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let live = spawn_pty_retained(
        "/bin/sh",
        &[
            "-c".into(),
            "while read line; do printf 'PROCESSED[%s]\\n' \"$line\"; done".into(),
        ],
        None,
        &[],
    )
    .unwrap();
    let mut rx = live.subscribe();
    registry.insert("term-dev".to_string(), live);
    routing.register_agent("sess-dev", "dev", "term-dev", "2000-01-01T00:00:00Z");

    deliver_to_terminals(&chat("pm", "reach-the-role", None), &registry, &routing);

    let seen = drain_for(&mut rx, "PROCESSED[").await;
    assert!(
        seen.contains("PROCESSED[") && seen.contains("reach-the-role"),
        "the \\r-submit makes a bare read-loop RETURN from `read` and PROCESS the line — it reaches the role processed, NOT parked awaiting a manual ENTER (falsifier-4, the 2026-06-06 reject, at the AGNOSTIC level: a non-Claude script, zero glue). Without the submit, `read` would block forever and PROCESSED[ would never appear; got {seen:?}"
    );
}

#[tokio::test]
async fn a_scoped_message_reaches_only_the_matching_display_no_leak() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let mut dev_rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");
    let mut pm_rx = register_live_cat(&registry, &routing, "sess-pm", "pm", "term-pm");

    deliver_to_terminals(
        &chat("designer", "for dev only", Some(vec!["dev".to_string()])),
        &registry,
        &routing,
    );

    let dev_seen = drain_for(&mut dev_rx, "for dev only").await;
    assert!(
        dev_seen.contains("⟦bpmux⟧ designer →you▸") && dev_seen.contains("for dev only"),
        "a to:[dev] message resolves dev's DISPLAY to its id and reaches dev's terminal, scoped →you; got {dev_seen:?}"
    );

    registry.write_to("term-pm", b"PM-SENTINEL\r");
    let pm_seen = drain_for(&mut pm_rx, "PM-SENTINEL").await;
    assert!(
        !pm_seen.contains("for dev only"),
        "no-leak: a message scoped to a DIFFERENT display must NOT reach pm's terminal (the sentinel barrier proves nothing arrived before it); got {pm_seen:?}"
    );
}

#[tokio::test]
async fn the_sender_does_not_receive_an_echo_of_its_own_message() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let mut dev_rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

    deliver_to_terminals(&chat("dev", "my own line", None), &registry, &routing);

    registry.write_to("term-dev", b"OWN-SENTINEL\r");
    let seen = drain_for(&mut dev_rx, "OWN-SENTINEL").await;
    assert!(
        !seen.contains("my own line"),
        "own-sender suppression: a broadcast must not be injected back into the SENDER's own terminal (no self-echo loop); got {seen:?}"
    );
}

#[tokio::test]
async fn an_agents_own_broadcast_under_its_bridge_name_is_not_reflected_back_into_its_own_terminal()
{
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let dev_terminal = "ccb4298d-1111-2222-3333-444455556666";
    let mut dev_rx = register_live_cat(&registry, &routing, "sess-dev", "dev", dev_terminal);
    let mut pm_rx = register_live_cat(&registry, &routing, "sess-pm", "pm", "term-pm");

    let sender = crate::session::routing_identity::bridge_agent_name(dev_terminal);
    deliver_to_terminals(&chat(&sender, "my own line", None), &registry, &routing);

    let pm_seen = drain_for(&mut pm_rx, "my own line").await;
    assert!(
        pm_seen.contains("⟦bpmux⟧") && pm_seen.contains("my own line"),
        "non-vacuous: a DIFFERENT agent's real terminal still receives the broadcast through the live delivery path; got {pm_seen:?}"
    );

    registry.write_to(dev_terminal, b"OWN-SENTINEL\r");
    let dev_seen = drain_for(&mut dev_rx, "OWN-SENTINEL").await;
    assert!(
        !dev_seen.contains("my own line"),
        "an agent's OWN broadcast is never reflected back into its own terminal — even though the bridge sends it under the auto-assigned name agent-{{terminal_id[..8]}} (NOT the dir-session display 'dev'), the live delivery path excludes the sender's own terminal. The sentinel arrives, proving we drained past where an echo would be; got {dev_seen:?}"
    );
}

#[tokio::test]
async fn a_register_in_a_dir_with_populated_history_starts_clean_no_backlog_dump() {
    let store_dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(store_dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let sess_dir = tempfile::TempDir::new().unwrap();
    let (tx, _rx) = broadcast::channel(16);
    let session =
        crate::session::Session::new(sess_dir.path().to_path_buf(), "owner".to_string(), tx);

    session
        .broadcast(&chat_at(
            "pm",
            "HISTORY-BROADCAST",
            None,
            "2020-01-01T00:00:00Z",
        ))
        .unwrap();
    session
        .broadcast(&chat_at(
            "designer",
            "HISTORY-ADDRESSED",
            Some(vec!["dev".to_string()]),
            "2020-01-02T00:00:00Z",
        ))
        .unwrap();

    let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
    let mut rx = live.subscribe();
    registry.insert("term-dev".to_string(), live);
    routing.register("sess-dev", "dev");
    routing.link_terminal("sess-dev", "term-dev");

    drain_missed(
        &session,
        &registry,
        &routing,
        "sess-dev",
        "term-dev",
        "dev",
        "2026-06-12T18:00:00Z",
    );

    registry.write_to("term-dev", b"CLEAN-SENTINEL\r");
    let seen = drain_for(&mut rx, "CLEAN-SENTINEL").await;
    assert!(
        !seen.contains("HISTORY-BROADCAST") && !seen.contains("HISTORY-ADDRESSED"),
        "registering an agent in a dir with REAL populated history starts the terminal CLEAN — the whole-history backlog is NEVER dumped (the disaster): broadcasts are live-forward-only (skipped) and addressed-before-JOIN is not history-dumped (bounded at join). Even with an empty cursor (the epoch-cursor bug), nothing historical reaches the terminal; got {seen:?}"
    );
}

#[tokio::test]
async fn a_message_in_the_join_gate_window_is_drained_addressed_or_the_welcome_broadcast() {
    let store_dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(store_dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let sess_dir = tempfile::TempDir::new().unwrap();
    let (tx, _rx) = broadcast::channel(16);
    let session =
        crate::session::Session::new(sess_dir.path().to_path_buf(), "owner".to_string(), tx);

    session
        .broadcast(&chat_at(
            "pm",
            "GATE-ADDRESSED",
            Some(vec!["dev".to_string()]),
            "2026-06-12T18:00:01Z",
        ))
        .unwrap();
    session
        .broadcast(&chat_at(
            "pm",
            "GATE-BROADCAST",
            None,
            "2026-06-12T18:00:02Z",
        ))
        .unwrap();

    let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
    let mut rx = live.subscribe();
    registry.insert("term-dev".to_string(), live);
    routing.register("sess-dev", "dev");
    routing.link_terminal("sess-dev", "term-dev");

    drain_missed(
        &session,
        &registry,
        &routing,
        "sess-dev",
        "term-dev",
        "dev",
        "2026-06-12T18:00:00Z",
    );

    registry.write_to("term-dev", b"GATE-SENTINEL\r");
    let seen = drain_for(&mut rx, "GATE-SENTINEL").await;
    assert!(
        seen.contains("GATE-ADDRESSED"),
        "an ADDRESSED message sent in the ~1s join-gate-window (after job-start) still drains on register — falsifier-4 intact; got {seen:?}"
    );
    assert!(
        seen.contains("GATE-BROADCAST"),
        "a BROADCAST sent in the join-gate-window — AFTER the agent's own job-start (the owner's 'welcome, i started you' the instant the agent spawned) — drains on register too: it is dropped by live delivery (the terminal isn't a delivery-target until the bind-gate) so the catch-up is its only path. Bounded by job-start, so this is NOT a history-dump and NOT a reconnect catch-up (those stay live-forward-only); got {seen:?}"
    );
}

#[tokio::test]
async fn offline_broadcasts_are_not_re_drained_on_reconnect_broadcasts_are_live_forward_only() {
    let store_dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(store_dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let sess_dir = tempfile::TempDir::new().unwrap();
    let (tx, _rx) = broadcast::channel(16);
    let session =
        crate::session::Session::new(sess_dir.path().to_path_buf(), "owner".to_string(), tx);

    let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
    let mut rx1 = live.subscribe();
    registry.insert("term1".to_string(), live);
    routing.register_agent("sess-dev", "dev", "term1", "2000-01-01T00:00:00Z");

    let m1 = chat_at("pm", "M1-delivered-live", None, "2026-06-13T10:00:00Z");
    session.broadcast(&m1).unwrap();
    deliver_to_terminals(&m1, &registry, &routing);
    assert!(
        drain_for(&mut rx1, "M1-delivered-live")
            .await
            .contains("M1-delivered-live"),
        "M1 reaches dev live while online"
    );

    registry.remove("term1");

    let m2 = chat_at("designer", "M2-while-offline", None, "2026-06-13T10:05:00Z");
    session.broadcast(&m2).unwrap();
    deliver_to_terminals(&m2, &registry, &routing);

    let live2 = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
    let mut rx2 = live2.subscribe();
    registry.insert("term2".to_string(), live2);
    routing.register_agent("sess-dev", "dev", "term2", "2026-06-13T11:00:00Z");
    drain_missed(
        &session,
        &registry,
        &routing,
        "sess-dev",
        "term2",
        "dev",
        "2026-06-13T11:00:00Z",
    );

    registry.write_to("term2", b"RECONNECT-SENTINEL\r");
    let drained = drain_for(&mut rx2, "RECONNECT-SENTINEL").await;
    assert!(
        !drained.contains("M2-while-offline") && !drained.contains("M1-delivered-live"),
        "a reconnecting agent does NOT catch up on broadcasts sent BEFORE its reconnect job-start — neither the live-delivered M1 (10:00) nor the offline-broadcast M2 (10:05) is re-dumped when the agent reconnects at 11:00; the gate-window catch-up is bounded by job-start, so ambient/offline broadcasts stay live-forward-only and the reconnect starts clean (the conscious falsifier-3 relaxation per owner); got {drained:?}"
    );
}

#[tokio::test]
async fn a_pty_loop_delivered_message_is_not_systematically_re_drained_by_the_dev_stop_hook() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
    let _rx = live.subscribe();
    registry.insert("term-dev".to_string(), live);
    routing.register_agent("sess-dev", "dev", "term-dev", "2000-01-01T00:00:00Z");

    let m = chat("pm", "single-delivery", None);
    deliver_to_terminals(&m, &registry, &routing);

    let floor = routing
        .drain_floor("sess-dev", Some("2020-01-01T00:00:00Z"))
        .expect("floor");
    let floor_ts = chrono::DateTime::parse_from_rfc3339(&floor)
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert!(
        floor_ts >= m.ts,
        "after the PTY-loop delivers M and advances the shared #1712 id-cursor, the still-firing DEV Stop-hook's wait-for-event drain (drain_floor) clamps a lagging client cursor FORWARD to >= M — so first_pending_after(floor) excludes M and the Stop-hook does NOT re-deliver it: SYSTEMATIC no-double via the shared cursor (whichever delivers first advances it). The narrow concurrent race (both read the cursor before either advances) is bounded-as-DEV-transitional — prod post-#1658 runs ONLY the PTY-loop. floor {floor:?} vs M.ts {}",
        m.ts
    );
}

#[tokio::test]
async fn a_committed_dormant_agent_is_woken_when_its_gate_event_fires() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let commitments = CommitmentStore::new();
    let live = spawn_pty_retained(
        "/bin/sh",
        &[
            "-c".into(),
            "while read line; do printf 'PROCESSED[%s]\\n' \"$line\"; done".into(),
        ],
        None,
        &[],
    )
    .unwrap();
    let mut rx = live.subscribe();
    registry.insert("term-dev".to_string(), live);
    routing.register_agent("sess-dev", "dev", "term-dev", "2000-01-01T00:00:00Z");

    commitments.commit("dev", "ci_state_changed", None);

    wake_committed_terminals(
        &ci_event(crate::ci::CiState::Green, "2026-06-21T10:00:00Z"),
        &registry,
        &routing,
        &commitments,
    );

    let seen = drain_for(&mut rx, "PROCESSED[").await;
    assert!(
        seen.contains("PROCESSED[") && seen.contains("ci_state_changed"),
        "a committed CI gate-event SUBMITS a wake-frame into the dormant read-loop — the \\r makes `read` RETURN and PROCESS the line (the agent gets a turn), NOT merely buffer stdin (the #1592 non-vacuity gate). A dormant agent has no turn-boundary, and the chat-only gate at line 78 drops the state-delta — without push-wake the committed event would never reach it. got {seen:?}"
    );
}

#[tokio::test]
async fn the_live_delivery_loop_wakes_a_committed_dormant_agent_on_a_broadcast_ci_event() {
    let store_dir = tempfile::TempDir::new().unwrap();
    let routing = Arc::new(RoutingStore::load(store_dir.path().to_path_buf()));
    let registry = Arc::new(LivePtyRegistry::new());
    let commitments = Arc::new(CommitmentStore::new());
    let sess_dir = tempfile::TempDir::new().unwrap();
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(crate::session::Session::new(
        sess_dir.path().to_path_buf(),
        "owner".to_string(),
        tx,
    ));

    let live = spawn_pty_retained(
        "/bin/sh",
        &[
            "-c".into(),
            "while read line; do printf 'PROCESSED[%s]\\n' \"$line\"; done".into(),
        ],
        None,
        &[],
    )
    .unwrap();
    let mut rx = live.subscribe();
    registry.insert("term-dev".to_string(), live);
    routing.register_agent("sess-dev", "dev", "term-dev", "2000-01-01T00:00:00Z");
    commitments.commit("dev", "ci_state_changed", None);

    spawn_terminal_delivery_loop(
        session.clone(),
        registry.clone(),
        routing.clone(),
        commitments.clone(),
    );

    session
        .broadcast(&ci_event(crate::ci::CiState::Green, "2026-06-21T10:00:00Z"))
        .unwrap();

    let seen = drain_for(&mut rx, "PROCESSED[").await;
    assert!(
        seen.contains("PROCESSED[") && seen.contains("ci_state_changed"),
        "the REAL prod delivery loop — not the function in isolation — wakes the committed dormant agent when a CI state-delta is broadcast through the session: subscribe→loop→wake_committed_terminals→PTY-submit is wired end-to-end. This is the anti-phantom proof (#1380 / the #1751 ci-lag): a path that merely LOOKS wired but never wakes is exactly the defect this story closes; got {seen:?}"
    );
}

#[tokio::test]
async fn a_committed_wake_is_held_while_the_owner_types_then_delivers_on_pause() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = Arc::new(RoutingStore::load(dir.path().to_path_buf()));
    let registry = Arc::new(LivePtyRegistry::new());
    let commitments = CommitmentStore::new();
    let live = spawn_pty_retained(
        "/bin/sh",
        &[
            "-c".into(),
            "while read line; do printf 'PROCESSED[%s]\\n' \"$line\"; done".into(),
        ],
        None,
        &[],
    )
    .unwrap();
    let mut rx = live.subscribe();
    registry.insert("term-dev".to_string(), live);
    routing.register_agent("sess-dev", "dev", "term-dev", "2000-01-01T00:00:00Z");
    commitments.commit("dev", "ci_state_changed", None);

    registry.note_human_keystroke("term-dev");
    let held = wake_committed_terminals(
        &ci_event(crate::ci::CiState::Green, "2026-06-21T10:00:00Z"),
        &registry,
        &routing,
        &commitments,
    );
    assert_eq!(
        held.len(),
        1,
        "while the owner is mid-typing INTO the dormant agent, the wake is HELD not submitted — a wake shares chat's submit-isolation so the \\r never merges his partial line into one Claude turn"
    );

    for inject in held {
        spawn_deferred_inject(registry.clone(), routing.clone(), inject);
    }
    let seen = drain_for(&mut rx, "PROCESSED[").await;
    assert!(
        seen.contains("PROCESSED[") && seen.contains("ci_state_changed"),
        "once the owner pauses past the debounce, the held wake delivers as its OWN turn — bounded hold, never a park; got {seen:?}"
    );
}

#[tokio::test]
async fn an_uncommitted_agent_is_not_woken_by_an_ambient_gate_event() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let commitments = CommitmentStore::new();
    let mut rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

    wake_committed_terminals(
        &ci_event(crate::ci::CiState::Green, "2026-06-21T10:00:00Z"),
        &registry,
        &routing,
        &commitments,
    );

    registry.write_to("term-dev", b"UNCOMMITTED-SENTINEL\r");
    let seen = drain_for(&mut rx, "UNCOMMITTED-SENTINEL").await;
    assert!(
        !seen.contains("ci_state_changed") && !seen.contains("fire-on-event"),
        "an agent that did NOT commit is never woken by an ambient state event — push-wake fires ONLY for a registered commitment, so the chat-only drop at line 78 still holds for every uncommitted terminal (no ambient-event spam). The sentinel proves we drained past where a wake would be; got {seen:?}"
    );
}

#[tokio::test]
async fn a_filtered_commitment_is_not_woken_by_a_non_matching_event_at_the_delivery_seam() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let commitments = CommitmentStore::new();
    let mut rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

    let mut filter = serde_json::Map::new();
    filter.insert("conclusion".to_string(), serde_json::json!("red"));
    commitments.commit("dev", "ci_state_changed", Some(filter));

    wake_committed_terminals(
        &ci_event(crate::ci::CiState::Green, "2026-06-21T10:00:00Z"),
        &registry,
        &routing,
        &commitments,
    );

    registry.write_to("term-dev", b"FILTER-SENTINEL\r");
    let seen = drain_for(&mut rx, "FILTER-SENTINEL").await;
    assert!(
        !seen.contains("ci_state_changed"),
        "a commitment filtered to conclusion:red is NOT woken by a GREEN CI event — the filter gates the wake at the live delivery seam, not only in the store unit. The armed commitment waits for the matching red event; got {seen:?}"
    );
}

#[tokio::test]
async fn a_non_chat_message_is_never_injected_into_a_terminal() {
    let dir = tempfile::TempDir::new().unwrap();
    let routing = RoutingStore::load(dir.path().to_path_buf());
    let registry = LivePtyRegistry::new();
    let mut rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

    let mut state = chat("bpmux", "STATE-BODY", None);
    state.message_type = MessageType::StateSnapshot;
    deliver_to_terminals(&state, &registry, &routing);

    registry.write_to("term-dev", b"NONCHAT-SENTINEL\r");
    let seen = drain_for(&mut rx, "NONCHAT-SENTINEL").await;
    assert!(
        !seen.contains("STATE-BODY"),
        "only chat/insight messages are injected — a state-snapshot must never be written into a PTY; got {seen:?}"
    );
}
