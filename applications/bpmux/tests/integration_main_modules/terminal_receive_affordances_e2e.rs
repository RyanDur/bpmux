#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use serde_json::json;

use crate::harness::pty::TestBpmux;

struct FakeAgentFarm {
    root: tempfile::TempDir,
}

impl FakeAgentFarm {
    fn new() -> Self {
        let root = tempfile::TempDir::new().expect("farm root");
        for sub in ["claim", "work", "capture"] {
            std::fs::create_dir_all(root.path().join(sub)).expect("farm subdir");
        }
        Self { root }
    }

    fn capture_path(&self, role: &str) -> std::path::PathBuf {
        self.root.path().join("capture").join(role)
    }

    fn write_agent_script(&self) -> std::path::PathBuf {
        let claim = self.root.path().join("claim");
        let work = self.root.path().join("work");
        let capture = self.root.path().join("capture");
        let script = format!(
            "#!/bin/bash\n\
             set -m\n\
             role=\"\"\n\
             for r in dev pm designer; do\n\
             \x20 if mkdir \"{claim}/$r\" 2>/dev/null; then role=\"$r\"; break; fi\n\
             done\n\
             [ -z \"$role\" ] && exit 1\n\
             mkdir -p \"{work}/$role\"\n\
             cd \"{work}/$role\" || exit 1\n\
             printf '\\033]7;file://localhost%s\\007' \"$PWD\"\n\
             cat >> \"{capture}/$role\"\n",
            claim = claim.display(),
            work = work.display(),
            capture = capture.display(),
        );
        let path = self.root.path().join("fake-agent.sh");
        std::fs::write(&path, script).expect("write fake agent");
        let mut perms = std::fs::metadata(&path).expect("agent meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod agent");
        path
    }

    fn write_register_agent_script(&self) -> std::path::PathBuf {
        let claim = self.root.path().join("claim");
        let work = self.root.path().join("work");
        let capture = self.root.path().join("capture");
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let script = format!(
            "#!/bin/bash\n\
             set -m\n\
             role=\"\"\n\
             for r in dev pm designer; do\n\
             \x20 if mkdir \"{claim}/$r\" 2>/dev/null; then role=\"$r\"; break; fi\n\
             done\n\
             [ -z \"$role\" ] && exit 1\n\
             mkdir -p \"{work}/$role\"\n\
             cd \"{work}/$role\" || exit 1\n\
             export BPMUX_ROLE=\"$role\"\n\
             export BPMUX_SESSION_ID=\"pinned-$role\"\n\
             \"{bin}\" register >> \"{capture}/$role.reg\" 2>&1\n\
             cat >> \"{capture}/$role\"\n",
            claim = claim.display(),
            work = work.display(),
            capture = capture.display(),
            bin = bin,
        );
        let path = self.root.path().join("register-agent.sh");
        std::fs::write(&path, script).expect("write register agent");
        let mut perms = std::fs::metadata(&path).expect("agent meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod agent");
        path
    }

    fn write_osc7_register_agent_script(&self) -> std::path::PathBuf {
        let claim = self.root.path().join("claim");
        let work = self.root.path().join("work");
        let capture = self.root.path().join("capture");
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let script = format!(
            "#!/bin/bash\n\
             set -m\n\
             role=\"\"\n\
             for r in dev pm designer; do\n\
             \x20 if mkdir \"{claim}/$r\" 2>/dev/null; then role=\"$r\"; break; fi\n\
             done\n\
             [ -z \"$role\" ] && exit 1\n\
             mkdir -p \"{work}/$role\"\n\
             cd \"{work}/$role\" || exit 1\n\
             export BPMUX_ROLE=\"$role\"\n\
             export BPMUX_SESSION_ID=\"pinned-$role\"\n\
             printf '\\033]7;file://localhost%s\\007' \"$PWD\"\n\
             \"{bin}\" register >> \"{capture}/$role.reg\" 2>&1\n\
             cat >> \"{capture}/$role\"\n",
            claim = claim.display(),
            work = work.display(),
            capture = capture.display(),
            bin = bin,
        );
        let path = self.root.path().join("osc7-register-agent.sh");
        std::fs::write(&path, script).expect("write osc7 register agent");
        let mut perms = std::fs::metadata(&path).expect("agent meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod agent");
        path
    }

    fn write_dir_default_then_register_agent_script(&self) -> std::path::PathBuf {
        let claim = self.root.path().join("claim");
        let work = self.root.path().join("work");
        let capture = self.root.path().join("capture");
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let script = format!(
            "#!/bin/bash\n\
             set -m\n\
             role=\"\"\n\
             for r in dev pm designer; do\n\
             \x20 if mkdir \"{claim}/$r\" 2>/dev/null; then role=\"$r\"; break; fi\n\
             done\n\
             [ -z \"$role\" ] && exit 1\n\
             mkdir -p \"{work}/bpmux-$role\"\n\
             cd \"{work}/bpmux-$role\" || exit 1\n\
             export BPMUX_ROLE=\"$role\"\n\
             printf '\\033]7;file://localhost%s\\007' \"$PWD\"\n\
             \"{bin}\" register >> \"{capture}/$role.reg\" 2>&1\n\
             cat >> \"{capture}/$role\"\n",
            claim = claim.display(),
            work = work.display(),
            capture = capture.display(),
            bin = bin,
        );
        let path = self.root.path().join("dir-default-register-agent.sh");
        std::fs::write(&path, script).expect("write dir-default register agent");
        let mut perms = std::fs::metadata(&path).expect("agent meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod agent");
        path
    }

    fn write_foreground_agent_script(&self) -> std::path::PathBuf {
        let claim = self.root.path().join("claim");
        let work = self.root.path().join("work");
        let capture = self.root.path().join("capture");
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let script = format!(
            "#!/bin/bash\n\
             set -m\n\
             role=\"\"\n\
             for r in dev pm designer; do\n\
             \x20 if mkdir \"{claim}/$r\" 2>/dev/null; then role=\"$r\"; break; fi\n\
             done\n\
             [ -z \"$role\" ] && exit 1\n\
             mkdir -p \"{work}/$role\"\n\
             cd \"{work}/$role\" || exit 1\n\
             export BPMUX_ROLE=\"$role\"\n\
             export BPMUX_SESSION_ID=\"pinned-$role\"\n\
             printf '\\033]7;file://localhost%s\\007' \"$PWD\"\n\
             \"{bin}\" register >> \"{capture}/$role.reg\" 2>&1\n\
             sh -c 'while [ ! -f \"$1\" ]; do sleep 0.1; done' _ \"{capture}/$role.die\"\n\
             exec sleep 3600\n",
            claim = claim.display(),
            work = work.display(),
            capture = capture.display(),
            bin = bin,
        );
        let path = self.root.path().join("foreground-agent.sh");
        std::fs::write(&path, script).expect("write foreground agent");
        let mut perms = std::fs::metadata(&path).expect("agent meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod agent");
        path
    }

    fn write_detect_then_cross_cwd_register_script(&self) -> std::path::PathBuf {
        let claim = self.root.path().join("claim");
        let work = self.root.path().join("work");
        let capture = self.root.path().join("capture");
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let script = format!(
            "#!/bin/bash\n\
             set -m\n\
             role=\"\"\n\
             for r in dev pm designer; do\n\
             \x20 if mkdir \"{claim}/$r\" 2>/dev/null; then role=\"$r\"; break; fi\n\
             done\n\
             [ -z \"$role\" ] && exit 1\n\
             mkdir -p \"{work}/bpmux-$role\"\n\
             cd \"{work}/bpmux-$role\" || exit 1\n\
             export BPMUX_ROLE=\"$role\"\n\
             printf '\\033]7;file://localhost%s\\007' \"$PWD\"\n\
             sleep 1.5\n\
             export BPMUX_SESSION_ID=\"pinned-other-$role\"\n\
             \"{bin}\" register >> \"{capture}/$role.reg\" 2>&1\n\
             cat >> \"{capture}/$role\"\n",
            claim = claim.display(),
            work = work.display(),
            capture = capture.display(),
            bin = bin,
        );
        let path = self.root.path().join("detect-cross-cwd-register.sh");
        std::fs::write(&path, script).expect("write cross-cwd register agent");
        let mut perms = std::fs::metadata(&path).expect("agent meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod agent");
        path
    }

    fn signal_agent_exit(&self, role: &str) {
        std::fs::write(self.capture_path(role).with_extension("die"), "").expect("drop sentinel");
    }

    fn capture_contains(&self, role: &str, needle: &str) -> bool {
        std::fs::read_to_string(self.capture_path(role))
            .map(|s| s.contains(needle))
            .unwrap_or(false)
    }

    fn capture_count(&self, role: &str, needle: &str) -> usize {
        std::fs::read_to_string(self.capture_path(role))
            .map(|s| s.matches(needle).count())
            .unwrap_or(0)
    }
}

async fn spawn_terminal(bpmux: &TestBpmux) -> String {
    let resp = bpmux
        .authed_post("/api/terminals/spawn", json!({}))
        .await
        .json::<serde_json::Value>()
        .await
        .expect("spawn json");
    resp["terminal_id"]
        .as_str()
        .expect("terminal_id")
        .to_string()
}

async fn post_message(bpmux: &TestBpmux, body: serde_json::Value) {
    let resp = bpmux.authed_post("/api/messages", body).await;
    assert!(resp.status().is_success(), "POST /api/messages failed");
}

async fn probe_until_live(bpmux: &TestBpmux, farm: &FakeAgentFarm, role: &str) -> bool {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
    let mut n = 0;
    while tokio::time::Instant::now() < deadline {
        n += 1;
        let marker = format!("READY-PROBE-{role}-{n}");
        post_message(bpmux, json!({ "from": "owner", "text": marker.clone() })).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
        if farm.capture_contains(role, &marker) {
            return true;
        }
    }
    false
}

async fn terminal_send(port: u16, role: &str, to: Option<&str>, text: &str) {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let mut args = vec!["send".to_string()];
    if let Some(to) = to {
        args.push("--to".to_string());
        args.push(to.to_string());
    }
    args.push(text.to_string());
    let out = tokio::process::Command::new(bin)
        .args(&args)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", role)
        .output()
        .await
        .expect("run bpmux send");
    assert!(
        out.status.success(),
        "bpmux send failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

async fn wait_capture(farm: &FakeAgentFarm, role: &str, needle: &str) -> bool {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    while tokio::time::Instant::now() < deadline {
        if farm.capture_contains(role, needle) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    farm.capture_contains(role, needle)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fake_agent_terminal_registers_and_receives_a_live_broadcast_injected_into_it() {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    let _terminal = spawn_terminal(&bpmux).await;

    let live = probe_until_live(&bpmux, &farm, "dev").await;

    assert!(
        live,
        "a fake-agent terminal must (1) trip the REAL agent-detect registration via its OSC-7 + sustained foreground `cat` job, then (2) receive a live broadcast INJECTED into its PTY by the real delivery loop — proving the agnostic terminal-receive affordance end-to-end through the live server, not a routing unit. capture(dev)={:?}",
        std::fs::read_to_string(farm.capture_path("dev")).unwrap_or_default()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_terminal_receive_affordance_routes_group_and_direct_and_withholds_what_is_not_for_you()
{
    let farm = FakeAgentFarm::new();
    let agent = farm.write_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    let dev_terminal = spawn_terminal(&bpmux).await;
    assert!(
        probe_until_live(&bpmux, &farm, "dev").await,
        "dev must register"
    );
    let _pm_terminal = spawn_terminal(&bpmux).await;
    assert!(
        probe_until_live(&bpmux, &farm, "pm").await,
        "pm must register"
    );
    let _designer_terminal = spawn_terminal(&bpmux).await;
    assert!(
        probe_until_live(&bpmux, &farm, "designer").await,
        "designer must register"
    );

    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "DIRECT-TO-DEV", "to": ["dev"] }),
    )
    .await;
    assert!(
        wait_capture(&farm, "dev", "DIRECT-TO-DEV").await,
        "a DIRECT message (to:[dev]) is injected into dev's terminal"
    );
    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "BARRIER-AFTER-DIRECT" }),
    )
    .await;
    assert!(
        wait_capture(&farm, "pm", "BARRIER-AFTER-DIRECT").await
            && wait_capture(&farm, "designer", "BARRIER-AFTER-DIRECT").await,
        "barrier broadcast must reach pm + designer, proving the delivery cycle advanced past DIRECT-TO-DEV"
    );
    assert!(
        !farm.capture_contains("pm", "DIRECT-TO-DEV")
            && !farm.capture_contains("designer", "DIRECT-TO-DEV"),
        "FILTERING: a direct message to:[dev] is WITHHELD from pm and designer — not their message, never injected into their terminal"
    );

    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "GROUP-DEV-PM", "to": ["dev", "pm"] }),
    )
    .await;
    assert!(
        wait_capture(&farm, "dev", "GROUP-DEV-PM").await
            && wait_capture(&farm, "pm", "GROUP-DEV-PM").await,
        "a GROUP message (to:[dev,pm]) is injected into BOTH addressed terminals"
    );
    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "BARRIER-AFTER-GROUP" }),
    )
    .await;
    assert!(
        wait_capture(&farm, "designer", "BARRIER-AFTER-GROUP").await,
        "barrier broadcast must reach designer, proving the cycle advanced past GROUP-DEV-PM"
    );
    assert!(
        !farm.capture_contains("designer", "GROUP-DEV-PM"),
        "FILTERING: a group message to:[dev,pm] is WITHHELD from designer (outside the group)"
    );

    let dev_bridge_name = bpmux::session::routing_identity::bridge_agent_name(&dev_terminal);
    post_message(
        &bpmux,
        json!({ "from": dev_bridge_name, "text": "DEVS-OWN-LINE" }),
    )
    .await;
    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "BARRIER-AFTER-SELF" }),
    )
    .await;
    assert!(
        wait_capture(&farm, "dev", "BARRIER-AFTER-SELF").await,
        "barrier broadcast must reach dev, proving the cycle advanced past dev's own broadcast"
    );
    assert!(
        !farm.capture_contains("dev", "DEVS-OWN-LINE"),
        "FILTERING (self-echo): dev's OWN broadcast — sent under its auto-assigned bridge name agent-{{terminal_id[..8]}}, NOT its display 'dev' — is never reflected back into dev's own terminal (the owner-reject)"
    );
    assert!(
        farm.capture_contains("pm", "DEVS-OWN-LINE")
            && farm.capture_contains("designer", "DEVS-OWN-LINE"),
        "non-vacuous: dev's broadcast DID reach the other terminals — only the sender's own is withheld"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_bpmux_send_terminal_command_routes_to_exactly_the_addressed_terminals_and_not_the_sender()
 {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );
    let port = bpmux.port();

    for role in ["dev", "pm", "designer"] {
        spawn_terminal(&bpmux).await;
        assert!(
            probe_until_live(&bpmux, &farm, role).await,
            "{role} must register"
        );
    }

    terminal_send(port, "dev", Some("pm"), "DEV-SENDS-TO-PM").await;
    post_message(&bpmux, json!({ "from": "owner", "text": "BARRIER-1" })).await;
    assert!(
        wait_capture(&farm, "pm", "DEV-SENDS-TO-PM").await,
        "vice-versa DIRECT: dev running `bpmux send --to pm` in its terminal reaches pm's terminal"
    );
    assert!(
        wait_capture(&farm, "dev", "BARRIER-1").await
            && wait_capture(&farm, "designer", "BARRIER-1").await,
        "owner barrier must reach dev + designer, proving the cycle advanced past the dev->pm send"
    );
    assert!(
        !farm.capture_contains("designer", "DEV-SENDS-TO-PM")
            && !farm.capture_contains("dev", "DEV-SENDS-TO-PM"),
        "FILTERING: `bpmux send --to pm` is WITHHELD from designer (not addressed) AND from dev (the sender never receives its own)"
    );

    terminal_send(port, "dev", Some("pm,designer"), "DEV-SENDS-GROUP").await;
    post_message(&bpmux, json!({ "from": "owner", "text": "BARRIER-2" })).await;
    assert!(
        wait_capture(&farm, "pm", "DEV-SENDS-GROUP").await
            && wait_capture(&farm, "designer", "DEV-SENDS-GROUP").await,
        "vice-versa GROUP: dev running `bpmux send --to pm,designer` reaches BOTH addressed terminals"
    );
    assert!(
        wait_capture(&farm, "dev", "BARRIER-2").await,
        "owner barrier must reach dev, proving the cycle advanced past dev's group send"
    );
    assert!(
        !farm.capture_contains("dev", "DEV-SENDS-GROUP"),
        "the sender (dev) never receives its own `bpmux send --to pm,designer`"
    );

    terminal_send(port, "dev", None, "DEV-SENDS-BROADCAST").await;
    post_message(&bpmux, json!({ "from": "owner", "text": "BARRIER-3" })).await;
    assert!(
        wait_capture(&farm, "pm", "DEV-SENDS-BROADCAST").await
            && wait_capture(&farm, "designer", "DEV-SENDS-BROADCAST").await,
        "vice-versa BROADCAST: dev running `bpmux send` reaches the other terminals"
    );
    assert!(
        wait_capture(&farm, "dev", "BARRIER-3").await,
        "owner barrier must reach dev, proving the cycle advanced past dev's broadcast"
    );
    assert!(
        !farm.capture_contains("dev", "DEV-SENDS-BROADCAST"),
        "dev's own broadcast via `bpmux send` is not reflected back into dev's own terminal"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn message_routing_runs_end_to_end_on_the_sqlite_backend_the_cutover_user_journey() {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[
            ("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8")),
            ("BPMUX_STORAGE_BACKEND", "sqlite"),
        ],
    );
    let port = bpmux.port();

    for role in ["dev", "pm", "designer"] {
        spawn_terminal(&bpmux).await;
        assert!(
            probe_until_live(&bpmux, &farm, role).await,
            "{role} registers and goes live on the SQLite-backed server — the agent-detect → register → agents-table write path runs clean on sqlite (the cutover substrate)"
        );
    }

    terminal_send(port, "dev", Some("pm"), "SQLITE-DIRECT-DEV-TO-PM").await;
    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "SQLITE-BARRIER-1" }),
    )
    .await;
    assert!(
        wait_capture(&farm, "pm", "SQLITE-DIRECT-DEV-TO-PM").await,
        "DIRECTED routing on sqlite: `bpmux send --to pm` resolves pm's display → id → terminal and injects into pm's terminal"
    );
    assert!(
        wait_capture(&farm, "dev", "SQLITE-BARRIER-1").await
            && wait_capture(&farm, "designer", "SQLITE-BARRIER-1").await,
        "the owner barrier reaches dev + designer, proving the delivery cycle advanced past the directed send"
    );
    assert!(
        !farm.capture_contains("designer", "SQLITE-DIRECT-DEV-TO-PM")
            && !farm.capture_contains("dev", "SQLITE-DIRECT-DEV-TO-PM"),
        "FILTERING on sqlite: the to:[pm] send is withheld from designer (unaddressed) AND from dev (the sender never receives its own)"
    );

    terminal_send(port, "dev", None, "SQLITE-BROADCAST").await;
    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "SQLITE-BARRIER-2" }),
    )
    .await;
    assert!(
        wait_capture(&farm, "pm", "SQLITE-BROADCAST").await
            && wait_capture(&farm, "designer", "SQLITE-BROADCAST").await,
        "BROADCAST routing on sqlite: dev's `bpmux send` reaches every other live terminal"
    );
    assert!(
        wait_capture(&farm, "dev", "SQLITE-BARRIER-2").await,
        "the owner barrier reaches dev, proving the cycle advanced past dev's broadcast"
    );
    assert!(
        !farm.capture_contains("dev", "SQLITE-BROADCAST"),
        "SELF-ECHO on sqlite: dev's own broadcast is never reflected back into dev's terminal"
    );

    use bpmux::session::storage::{SqliteStorage, Storage};
    let storage = SqliteStorage::new(bpmux.sessions_base_path().join("current"))
        .expect("open the server's sqlite session store");
    let mut roles: Vec<String> = storage
        .load_agents()
        .expect("load the agents table")
        .into_iter()
        .map(|record| record.role)
        .collect();
    roles.sort();
    roles.dedup();
    assert!(
        ["designer", "dev", "pm"]
            .iter()
            .all(|wanted| roles.iter().any(|role| role == wanted)),
        "SUBSTRATE: the three routed identities are durable rows in the SQLite agents table — registration wrote routing identity THROUGH to sqlite, so the database genuinely IS the routing store under the cutover. The startup rebuild-from-sqlite read path is unit-pinned by routing_identity::store_tests (reload via a fresh RoutingStore over the same storage). got {roles:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pinned_agent_that_runs_register_binds_its_one_id_to_its_terminal_for_addressed_delivery()
{
    let farm = FakeAgentFarm::new();
    let agent = farm.write_register_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    spawn_terminal(&bpmux).await;
    assert!(
        probe_until_live(&bpmux, &farm, "dev").await,
        "dev registers via `bpmux register` ALONE (no OSC-7 emit, so agent-detect never mints) — receiving a broadcast proves the terminal was bound through register's forwarded BPMUX_TERMINAL_ID. register output: {:?}",
        std::fs::read_to_string(farm.root.path().join("capture").join("dev.reg"))
            .unwrap_or_default()
    );
    spawn_terminal(&bpmux).await;
    assert!(
        probe_until_live(&bpmux, &farm, "pm").await,
        "pm must register"
    );

    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "ADDRESSED-TO-DEV", "to": ["dev"] }),
    )
    .await;
    assert!(
        wait_capture(&farm, "dev", "ADDRESSED-TO-DEV").await,
        "an ADDRESSED message (to:[dev]) reaches dev's terminal — because `bpmux register` forwarded BPMUX_TERMINAL_ID so the server bound dev's ONE pinned id (pinned-dev) to its terminal. This is the addressed-receive that was BROKEN when the terminal was not bound to the id the message routes to"
    );
    post_message(&bpmux, json!({ "from": "owner", "text": "BARRIER" })).await;
    assert!(
        wait_capture(&farm, "pm", "BARRIER").await,
        "barrier reaches pm, proving the delivery cycle advanced past the dev-addressed message"
    );
    assert!(
        !farm.capture_contains("pm", "ADDRESSED-TO-DEV"),
        "FILTERING: the to:[dev] message is withheld from pm"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_osc7_emitting_agent_that_also_registers_is_bound_once_no_duplicate_injection() {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_osc7_register_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    spawn_terminal(&bpmux).await;
    assert!(
        probe_until_live(&bpmux, &farm, "dev").await,
        "dev registers (via `bpmux register`) and receives"
    );
    tokio::time::sleep(Duration::from_secs(2)).await;

    post_message(
        &bpmux,
        json!({ "from": "owner", "text": "SINGLE-COPY-SENTINEL" }),
    )
    .await;
    assert!(
        wait_capture(&farm, "dev", "SINGLE-COPY-SENTINEL").await,
        "the broadcast reaches dev's terminal"
    );
    post_message(&bpmux, json!({ "from": "owner", "text": "COUNT-BARRIER" })).await;
    assert!(
        wait_capture(&farm, "dev", "COUNT-BARRIER").await,
        "the barrier reaches dev, proving this broadcast's delivery cycle is complete"
    );
    assert_eq!(
        farm.capture_count("dev", "SINGLE-COPY-SENTINEL"),
        1,
        "the agent emits OSC-7 (so agent-detect WOULD mint a dir-derived id) AND registers its pinned id — agent-detect must DEFER because the terminal is already linked, so the terminal carries ONE id, not two. Two ids both bound to the one terminal would inject the broadcast TWICE; capture={:?}",
        std::fs::read_to_string(farm.capture_path("dev")).unwrap_or_default()
    );
}

async fn roster_identities(bpmux: &TestBpmux) -> Vec<String> {
    let url = format!("http://127.0.0.1:{}/api/roster", bpmux.port());
    let body: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .bearer_auth(bpmux.simulate_secret())
        .header(reqwest::header::HOST, format!("127.0.0.1:{}", bpmux.port()))
        .send()
        .await
        .expect("GET /api/roster")
        .json()
        .await
        .expect("roster json");
    body["roster"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|e| e["identity"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

async fn wait_roster<F: Fn(&[String]) -> bool>(bpmux: &TestBpmux, pred: F) -> Vec<String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut ids = roster_identities(bpmux).await;
    while tokio::time::Instant::now() < deadline {
        if pred(&ids) {
            return ids;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        ids = roster_identities(bpmux).await;
    }
    ids
}

#[tokio::test(flavor = "multi_thread")]
async fn a_foreground_agent_appears_then_leaves_when_its_job_ends_while_the_terminal_stays_alive() {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_foreground_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    let _terminal_id = spawn_terminal(&bpmux).await;
    let present = wait_roster(&bpmux, |ids| ids.iter().any(|i| i == "dev")).await;
    assert!(
        present.iter().any(|i| i == "dev"),
        "a live agent — the foreground job in a bpmux terminal — is DETECTED and joins the roster (presence = an agent is the foreground job, derived from the PTY bpmux owns); got {present:?}"
    );

    tokio::time::sleep(Duration::from_secs(2)).await;
    let still = roster_identities(&bpmux).await;
    assert!(
        still.iter().any(|i| i == "dev"),
        "an IDLE foreground agent STAYS — it is still the foreground job (it must be, to hold the tty), so presence holds with no signal from the agent. got {still:?}"
    );

    farm.signal_agent_exit("dev");
    let after = wait_roster(&bpmux, |ids| !ids.iter().any(|i| i == "dev")).await;
    assert!(
        !after.iter().any(|i| i == "dev"),
        "when the agent exits but the terminal stays alive, the foreground returns to the shell and bpmux LEAVES it — presence tracks the AGENT (foreground job), never the bare pane. got {after:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_closed_terminal_drops_its_foreground_agent() {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_foreground_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    let terminal_id = spawn_terminal(&bpmux).await;
    assert!(
        wait_roster(&bpmux, |ids| ids.iter().any(|i| i == "dev"))
            .await
            .iter()
            .any(|i| i == "dev"),
        "the foreground agent is detected and present"
    );

    bpmux
        .authed_post(
            "/api/terminals/close",
            json!({ "terminal_id": terminal_id, "force": true }),
        )
        .await;
    let after = wait_roster(&bpmux, |ids| !ids.iter().any(|i| i == "dev")).await;
    assert!(
        !after.iter().any(|i| i == "dev"),
        "closing the terminal (pane-close) tears down the PTY bpmux owns → the agent leaves the roster — the other death-mode, also handled by the foreground-job detector. got {after:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_dir_name_default_collapses_into_the_registered_role_one_id_keyed_row_no_dup() {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_dir_default_then_register_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    let _terminal_id = spawn_terminal(&bpmux).await;
    assert!(
        probe_until_live(&bpmux, &farm, "dev").await,
        "the agent auto-detects (dir-name default) AND registers its role, then is reachable as 'dev'"
    );

    let roster = wait_roster(&bpmux, |ids| ids.iter().any(|i| i == "dev")).await;
    let dev_rows: Vec<&String> = roster.iter().filter(|i| i.contains("dev")).collect();
    assert_eq!(
        dev_rows.len(),
        1,
        "the dir-name default and the registered role are ONE id-keyed row, not two — this is owner's dup screenshot (the gray 'bpmux-dev' default beside the registered 'dev') made impossible by routing-id keying; got roster {roster:?}"
    );
    assert_eq!(
        dev_rows[0], "dev",
        "register REPLACED the dir-name default ('bpmux-dev') with the set display ('dev') in place on the same routing-id row"
    );
    assert!(
        !roster.iter().any(|i| i == "bpmux-dev"),
        "the dir-name default leaves NO orphan row once the role is set — the placeholder is shown only until the display name is set; got {roster:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cross_cwd_register_reaps_the_detection_default_so_any_cwd_is_one_row() {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_detect_then_cross_cwd_register_script();
    let bpmux = TestBpmux::start_with_envs(
        "owner",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    let _terminal_id = spawn_terminal(&bpmux).await;
    assert!(
        probe_until_live(&bpmux, &farm, "dev").await,
        "the detection mints the dir-default 'bpmux-dev' first, then the agent registers its role from a DIFFERENT session-id (a different cwd), then is reachable as 'dev'"
    );

    let roster = wait_roster(&bpmux, |ids| ids.iter().any(|i| i == "dev")).await;
    assert!(
        roster.iter().any(|i| i == "dev"),
        "the registered role 'dev' is present; got {roster:?}"
    );
    assert!(
        !roster.iter().any(|i| i == "bpmux-dev"),
        "registering from a cwd whose session-id differs from the terminal's DETECTION session-id STILL collapses to one row — register forwards BPMUX_TERMINAL_ID, so it reaps the orphaned detection dir-default ('bpmux-dev') when it takes the terminal over. Identity is terminal-anchored, not cwd-coupled (owner's 'you don't need to cd'); got {roster:?}"
    );
}
