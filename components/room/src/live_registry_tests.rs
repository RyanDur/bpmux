#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::live_registry::*;
use crate::spawn::spawn_pty_retained;
use tokio::sync::broadcast;

fn extract_pid(stream: &str) -> String {
    let at = stream
        .find("RPID=")
        .expect("the PTY output must carry the RPID= marker");
    stream[at + 5..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect()
}

fn drain_until(rx: &mut broadcast::Receiver<Vec<u8>>, marker: &str) -> String {
    let mut out = String::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        match rx.blocking_recv() {
            Ok(bytes) if bytes.is_empty() => break,
            Ok(bytes) => {
                out.push_str(&String::from_utf8_lossy(&bytes));
                if out.contains(marker) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    out
}

#[test]
fn registry_hands_out_a_working_subscriber_keyed_by_role() {
    let registry = LivePtyRegistry::new();
    let live = spawn_pty_retained("/bin/sh", &[], None, &[]).unwrap();
    registry.insert("dev".to_string(), live);

    assert!(registry.contains("dev"));
    assert!(!registry.contains("pm"));

    let mut rx = registry.subscribe_for("dev").expect("dev room present");
    registry.write_to("dev", b"printf 'registry-stream-marker\\n'\n");
    let collected = drain_until(&mut rx, "registry-stream-marker");
    assert!(
        collected.contains("registry-stream-marker"),
        "registry.subscribe_for must stream the role's live PTY output to a subscriber; got {collected:?}"
    );
}

#[test]
fn subscribe_for_absent_role_is_none() {
    let registry = LivePtyRegistry::new();
    assert!(registry.subscribe_for("nobody").is_none());
}

#[test]
fn registry_write_reaches_the_same_live_session_stdin_across_multiple_writes() {
    let registry = LivePtyRegistry::new();
    let live = spawn_pty_retained(
        "/bin/sh",
        &["-c".into(), "read line; printf 'GOT:%s' \"$line\"".into()],
        None,
        &[],
    )
    .unwrap();
    registry.insert("dev".to_string(), live);

    let mut rx = registry.subscribe_for("dev").expect("dev present");
    assert!(registry.write_to("dev", b"ping"));
    assert!(
        registry.write_to("dev", b"-stdin\n"),
        "the SECOND write must also reach stdin — the writer is taken once at spawn and reused, not re-taken per write"
    );

    let collected = drain_until(&mut rx, "GOT:ping-stdin");
    assert!(
        collected.contains("GOT:ping-stdin"),
        "writing to the room write-half must reach the SAME live session's stdin (the child reads it and echoes GOT:<line> on the same master); got {collected:?}"
    );
}

#[test]
fn write_to_absent_role_is_false() {
    let registry = LivePtyRegistry::new();
    assert!(!registry.write_to("nobody", b"x"));
}

#[test]
fn two_rooms_are_isolated_no_cross_bleed_and_both_coexist() {
    let registry = LivePtyRegistry::new();
    let dev = spawn_pty_retained("/bin/sh", &[], None, &[]).unwrap();
    let pm = spawn_pty_retained("/bin/sh", &[], None, &[]).unwrap();
    registry.insert("dev".to_string(), dev);
    registry.insert("pm".to_string(), pm);

    assert!(
        registry.contains("dev") && registry.contains("pm"),
        "spawning the pm room must not destabilize the dev room — both coexist"
    );

    let mut dev_rx = registry.subscribe_for("dev").expect("dev present");
    let mut pm_rx = registry.subscribe_for("pm").expect("pm present");
    registry.write_to("dev", b"printf 'DEV-ROOM-OUTPUT\\n'\n");
    registry.write_to("pm", b"printf 'PM-ROOM-OUTPUT\\n'\n");

    let dev_out = drain_until(&mut dev_rx, "DEV-ROOM-OUTPUT");
    let pm_out = drain_until(&mut pm_rx, "PM-ROOM-OUTPUT");

    assert!(dev_out.contains("DEV-ROOM-OUTPUT"));
    assert!(
        !dev_out.contains("PM-ROOM-OUTPUT"),
        "EXERCISED bleed-vector: pm's output must NOT reach the dev room; got {dev_out:?}"
    );
    assert!(pm_out.contains("PM-ROOM-OUTPUT"));
    assert!(
        !pm_out.contains("DEV-ROOM-OUTPUT"),
        "EXERCISED bleed-vector: dev's output must NOT reach the pm room; got {pm_out:?}"
    );
}

#[test]
fn reap_dead_and_list_live_returns_only_living_ids_and_reaps_the_exited_out_of_the_registry() {
    let registry = LivePtyRegistry::new();
    let living =
        spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[]).unwrap();
    let exiting =
        spawn_pty_retained("/bin/sh", &["-c".into(), "exit 0".into()], None, &[]).unwrap();
    registry.insert("living-terminal".to_string(), living);
    registry.insert("exited-terminal".to_string(), exiting);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let live = loop {
        let live = registry.reap_dead_and_list_live();
        if !live.iter().any(|id| id == "exited-terminal") {
            break live;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the exited terminal never dropped out of the live set"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    };

    assert!(
        live.iter().any(|id| id == "living-terminal"),
        "the still-running terminal must remain in the live set; got {live:?}"
    );
    assert!(
        registry.contains("living-terminal"),
        "reconciling must not evict a live terminal from the registry"
    );
    assert!(
        !registry.contains("exited-terminal"),
        "the exited terminal must be reaped OUT of the registry — that single pass closes the orphan-PTY leak (d) AND prevents the exited terminal resurrecting on the next refresh's re-attach (e)"
    );
}

#[test]
fn dropping_a_subscriber_then_resubscribing_reattaches_to_the_same_live_process() {
    let registry = LivePtyRegistry::new();
    let live = spawn_pty_retained("/bin/sh", &[], None, &[]).unwrap();
    registry.insert("dev".to_string(), live);

    let mut first = registry.subscribe_for("dev").expect("dev present");
    assert!(registry.write_to("dev", b"printf 'RPID''=%s\\n' $$\n"));
    let before = drain_until(&mut first, "RPID=");
    let pid_before = extract_pid(&before);
    assert!(
        !pid_before.is_empty(),
        "the first attach must observe the live shell's pid; got {before:?}"
    );
    drop(first);

    let mut second = registry.subscribe_for("dev").expect(
        "a refresh re-attaches: the live session must survive a dropped subscriber, not vanish",
    );
    assert!(registry.write_to("dev", b"printf 'RPID''=%s\\n' $$\n"));
    let after = drain_until(&mut second, "RPID=");
    let pid_after = extract_pid(&after);

    assert_eq!(
        pid_before, pid_after,
        "re-subscribing after a disconnect must re-attach to the SAME live process (a refresh re-attaches by terminal_id; it does NOT respawn a fresh shell); before={before:?} after={after:?}"
    );
}
