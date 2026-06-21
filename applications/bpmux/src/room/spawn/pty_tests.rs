#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::pty::{resolve_terminal_spawn, spawn_pty, spawn_pty_retained};
use crate::room::spawn::process_alive;
use tokio::sync::broadcast;

#[test]
fn spawn_pty_returns_positive_pid_for_running_child() {
    let pid = spawn_pty("/bin/sh", &["-c".into(), "sleep 1".into()]).unwrap();
    assert!(pid > 0, "spawn must return a child PID; got {pid}");
    assert!(
        process_alive(pid),
        "child {pid} must be alive immediately after spawn"
    );
}

#[test]
fn spawn_pty_survives_parent_handle_drop() {
    let pid = spawn_pty("/bin/sh", &["-c".into(), "sleep 2".into()]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(
        process_alive(pid),
        "child {pid} must outlive the spawn handles being dropped"
    );
}

#[test]
fn spawn_pty_retained_streams_child_output_to_a_subscriber() {
    let live = spawn_pty_retained("/bin/sh", &[], None, &[]).unwrap();
    assert!(live.pid() > 0, "retained-pty spawn must report a child PID");
    let mut rx = live.subscribe();
    live.write_input(b"printf 'hello-retained-pty\\n'\n")
        .unwrap();
    let collected = drain_subscriber_until(&mut rx, "hello-retained-pty");
    assert!(
        collected.contains("hello-retained-pty"),
        "the RETAINED master's server-owned reader must stream the child's live PTY output to a subscriber (not dropped like spawn_pty); got {collected:?}"
    );
}

fn drain_subscriber_until(rx: &mut broadcast::Receiver<Vec<u8>>, marker: &str) -> String {
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
fn two_subscribers_to_one_pty_both_receive_the_full_output() {
    let live = spawn_pty_retained("/bin/sh", &[], None, &[]).unwrap();
    let mut first = live.subscribe();
    let mut second = live.subscribe();
    live.write_input(b"printf 'fanout-marker\\n'\n").unwrap();

    let first_out = drain_subscriber_until(&mut first, "fanout-marker");
    let second_out = drain_subscriber_until(&mut second, "fanout-marker");

    assert!(
        first_out.contains("fanout-marker"),
        "subscriber one must receive the live PTY output via the server-owned reader fanout; got {first_out:?}"
    );
    assert!(
        second_out.contains("fanout-marker"),
        "subscriber two must ALSO receive the SAME full output — one server-owned reader fans out to every subscriber, never splitting the master fd between competing per-subscriber readers; got {second_out:?}"
    );
}

#[test]
fn the_output_sniffer_latches_paste_capable_when_the_agent_declares_bracketed_paste() {
    let live = spawn_pty_retained(
        "/bin/sh",
        &[
            "-c".into(),
            "read trigger; printf '\\033[?2004hPASTE-ON-MARKER\\n'; sleep 2".into(),
        ],
        None,
        &[],
    )
    .unwrap();
    assert!(
        !live.paste_capable(),
        "a freshly-spawned terminal is NOT paste-capable until the agent declares bracketed-paste"
    );
    let mut rx = live.subscribe();
    live.write_input(b"go\n").unwrap();
    let collected = drain_subscriber_until(&mut rx, "PASTE-ON-MARKER");
    assert!(
        collected.contains("PASTE-ON-MARKER"),
        "precondition: the child's output (incl the ESC[?2004h that precedes the marker in the same write) must have streamed; got {collected:?}"
    );
    assert!(
        live.paste_capable(),
        "after the agent emits ESC[?2004h, the LivePty output-sniffer must have latched paste-capable ON — so injected lines get bracketed-paste-framed and auto-submit (a real Enter) instead of parking in the agent's input until a manual keypress (the owner-walk reject); the latch holds even when the agent later toggles bracketed-paste OFF while busy"
    );
}

#[test]
fn a_paste_capable_tui_that_toggled_paste_off_still_gets_a_bracket_framed_submit() {
    let live = spawn_pty_retained(
        "/bin/sh",
        &[
            "-c".into(),
            "printf '\\033[?2004h'; sleep 0.1; printf '\\033[?2004l'; sleep 0.1; exec cat".into(),
        ],
        None,
        &[],
    )
    .unwrap();
    let mut rx = live.subscribe();
    let toggles = drain_subscriber_until(&mut rx, "\x1b[?2004l");
    assert!(
        toggles.contains("\x1b[?2004h") && toggles.contains("\x1b[?2004l"),
        "precondition: the agent DECLARED (ESC[?2004h) then DISABLED (ESC[?2004l) bracketed-paste — capability latched, current-mode now off; got {toggles:?}"
    );

    live.submit_input(b"reach-the-role").unwrap();

    let seen = drain_subscriber_until(&mut rx, "reach-the-role");
    assert!(
        seen.contains("[200~") && seen.contains("[201~") && seen.contains("reach-the-role"),
        "a paste-CAPABLE raw-TUI that has since toggled paste OFF (busy/rendering) must STILL \
             receive the delivery bracket-paste-framed (the [200~…[201~ markers wrap the body) — the \
             capability LATCHES, so the submit never depends on the flickering current paste-mode. \
             Without the latch, a delivery in the paste-off window ships plain + a bare \\r that a \
             busy raw-TUI PARKS awaiting a manual Enter (owner's 2026-06-13 intermittent reject); got {seen:?}"
    );
}

#[test]
fn a_keystroke_records_human_input_activity_but_an_inject_does_not() {
    let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
    assert!(
        live.human_input_age().is_none(),
        "a fresh terminal has no human-input activity yet"
    );

    live.submit_input(b"injected line").unwrap();
    assert!(
        live.human_input_age().is_none(),
        "an INJECT (submit_input) is NOT human keystroke-activity — clause (d) must never let an inject suppress its OWN hold-window (it would self-defeat the submit-isolation)"
    );

    live.write_input(b"x").unwrap();
    assert!(
        live.human_input_age().is_some(),
        "an owner KEYSTROKE (write_input) records human-input activity — the clause (d) hold-trigger"
    );
}

#[test]
fn the_current_paste_mode_tracks_the_toggle_off_while_capability_stays_latched() {
    let live = spawn_pty_retained(
        "/bin/sh",
        &[
            "-c".into(),
            "printf '\\033[?2004h'; sleep 0.1; printf '\\033[?2004l'; sleep 0.1; exec cat".into(),
        ],
        None,
        &[],
    )
    .unwrap();
    let mut rx = live.subscribe();
    let toggles = drain_subscriber_until(&mut rx, "\x1b[?2004l");
    assert!(
        toggles.contains("\x1b[?2004h") && toggles.contains("\x1b[?2004l"),
        "precondition: the agent declared (ESC[?2004h) then disabled (ESC[?2004l) bracketed-paste; got {toggles:?}"
    );

    assert!(
        live.paste_capable(),
        "capability LATCHES on — the injection-framing path must never depend on the flickering live mode (the paste-off window), so a busy raw-TUI never parks a bare-\\r delivery"
    );
    assert!(
        !live.current_bracketed_paste(),
        "the LIVE paste-mode tracks the toggle OFF — this is the renderer's cooked/raw input-ownership signal (#1656), DISTINCT from the latched framing capability: a raw-TUI that exited bracketed-paste hands the cooked line back to the renderer to own"
    );
}

#[test]
fn spawn_pty_retained_runs_a_real_shell_in_the_given_cwd_with_term() {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let cwd = std::env::temp_dir().join(format!("bpmux-shell-cwd-{}-{}", std::process::id(), "s1"));
    std::fs::create_dir_all(&cwd).unwrap();
    let marker = cwd.file_name().unwrap().to_string_lossy().to_string();
    let live = spawn_pty_retained(
        &shell,
        &[],
        Some(cwd.as_path()),
        &[("TERM", "xterm-256color")],
    )
    .unwrap();
    let mut rx = live.subscribe();
    live.write_input(b"echo TERM=$TERM PWD=$PWD\n").unwrap();
    let mut collected = String::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        match rx.blocking_recv() {
            Ok(bytes) if bytes.is_empty() => break,
            Ok(bytes) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if collected.contains("TERM=xterm-256color") && collected.contains(&marker) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let _ = std::fs::remove_dir_all(&cwd);
    assert!(
        collected.contains("TERM=xterm-256color"),
        "the spawned shell must run with TERM=xterm-256color (interpolated by a REAL shell, not the literal echo); got {collected:?}"
    );
    assert!(
        collected.contains(&marker),
        "the spawned shell must start in the given cwd (PWD must contain the spawn dir, not the backend's cwd); got {collected:?}"
    );
}

#[test]
fn live_pty_resize_propagates_the_winsize_to_the_shell_tty() {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let live = spawn_pty_retained(&shell, &[], None, &[("TERM", "xterm-256color")]).unwrap();
    live.resize(40, 120).unwrap();
    let mut rx = live.subscribe();
    live.write_input(b"stty size\n").unwrap();
    let collected = drain_subscriber_until(&mut rx, "40 120");
    assert!(
        collected.contains("40 120"),
        "stty size must report the RESIZED winsize (rows cols), proving master.resize propagated to the slave tty — not the spawn default 24 80; got {collected:?}"
    );
}

#[test]
fn resolve_terminal_spawn_defaults_to_a_login_shell_when_no_override() {
    let (command, args) = resolve_terminal_spawn(None, Some("/bin/zsh"));
    assert_eq!(
        command, "/bin/zsh",
        "with no override ＋ spawns the user's $SHELL — a bare login shell (the agent RUNNING in it is the registration signal, sourced by the always-on detector, not by this spawn-target)"
    );
    assert_eq!(
        args,
        vec!["-l".to_string()],
        "the $SHELL spawns as a LOGIN shell so it loads the user's environment"
    );
}

#[test]
fn resolve_terminal_spawn_honours_an_override_without_login_args() {
    let (command, args) = resolve_terminal_spawn(Some("/bin/echo"), Some("/bin/zsh"));
    assert_eq!(
        command, "/bin/echo",
        "an explicit BPMUX_ROOM_AGENT override wins over $SHELL — the spawn-command seam (the e2e harness pins a deterministic spawn-target; unset in prod, so prod ＋ is the bare $SHELL)"
    );
    assert!(
        args.is_empty(),
        "an override spawns as-is (no login args injected into an arbitrary target)"
    );
}

#[test]
fn resolve_terminal_spawn_treats_empty_env_as_unset() {
    let (command, args) = resolve_terminal_spawn(Some(""), Some(""));
    assert_eq!(
        command, "/bin/sh",
        "empty env vars fall back to /bin/sh, never spawn \"\""
    );
    assert!(args.is_empty());
}

#[test]
fn close_and_reap_terminates_a_running_process_no_orphan() {
    let live =
        spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[]).unwrap();
    let pid = live.pid();
    assert!(
        process_alive(pid),
        "the spawned process must be alive before close; pid {pid}"
    );
    live.close_and_reap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while process_alive(pid) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        !process_alive(pid),
        "close_and_reap must terminate the spawned process (SIGTERM→grace→SIGKILL→reap) — pid {pid} still alive is the orphan leak this slice closes"
    );
}

#[test]
fn has_foreground_job_is_false_for_a_bare_spawned_process() {
    let live =
        spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[]).unwrap();
    assert!(
        !live.has_foreground_job(),
        "a bare spawned process with no forked foreground command must report no foreground job — closing it needs no confirm-before-close"
    );
    live.close_and_reap();
}

#[test]
fn is_alive_reports_true_while_running_and_false_after_the_child_exits() {
    let mut running =
        spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[]).unwrap();
    assert!(
        running.is_alive(),
        "a still-running child must report is_alive=true"
    );
    running.close_and_reap();

    let mut exiting =
        spawn_pty_retained("/bin/sh", &["-c".into(), "exit 0".into()], None, &[]).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while exiting.is_alive() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        !exiting.is_alive(),
        "after the child exits, is_alive must report false — try_wait() observes the exit and reaps the zombie, where a kill -0 pid-probe would false-positive on the lingering zombie pid that refresh-resurrection (e) depends on excluding"
    );
}
