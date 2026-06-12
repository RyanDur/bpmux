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

    fn capture_contains(&self, role: &str, needle: &str) -> bool {
        std::fs::read_to_string(self.capture_path(role))
            .map(|s| s.contains(needle))
            .unwrap_or(false)
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
        post_message(bpmux, json!({ "from": "ryan", "text": marker.clone() })).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
        if farm.capture_contains(role, &marker) {
            return true;
        }
    }
    false
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
        "ryan",
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
        "ryan",
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
        json!({ "from": "ryan", "text": "DIRECT-TO-DEV", "to": ["dev"] }),
    )
    .await;
    assert!(
        wait_capture(&farm, "dev", "DIRECT-TO-DEV").await,
        "a DIRECT message (to:[dev]) is injected into dev's terminal"
    );
    post_message(
        &bpmux,
        json!({ "from": "ryan", "text": "BARRIER-AFTER-DIRECT" }),
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
        json!({ "from": "ryan", "text": "GROUP-DEV-PM", "to": ["dev", "pm"] }),
    )
    .await;
    assert!(
        wait_capture(&farm, "dev", "GROUP-DEV-PM").await
            && wait_capture(&farm, "pm", "GROUP-DEV-PM").await,
        "a GROUP message (to:[dev,pm]) is injected into BOTH addressed terminals"
    );
    post_message(
        &bpmux,
        json!({ "from": "ryan", "text": "BARRIER-AFTER-GROUP" }),
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
        json!({ "from": "ryan", "text": "BARRIER-AFTER-SELF" }),
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
async fn a_message_sent_from_a_terminal_routes_to_exactly_the_addressed_terminals_and_not_the_sender()
 {
    let farm = FakeAgentFarm::new();
    let agent = farm.write_agent_script();
    let bpmux = TestBpmux::start_with_envs(
        "ryan",
        &[("BPMUX_ROOM_AGENT", agent.to_str().expect("agent path utf8"))],
    );

    for role in ["dev", "pm", "designer"] {
        spawn_terminal(&bpmux).await;
        assert!(
            probe_until_live(&bpmux, &farm, role).await,
            "{role} must register"
        );
    }

    post_message(
        &bpmux,
        json!({ "from": "dev", "text": "DEV-TO-PM", "to": ["pm"] }),
    )
    .await;
    post_message(&bpmux, json!({ "from": "dev", "text": "BARRIER-1" })).await;
    assert!(
        wait_capture(&farm, "pm", "DEV-TO-PM").await,
        "vice-versa DIRECT: a message dev sends to:[pm] is injected into pm's terminal"
    );
    assert!(
        wait_capture(&farm, "designer", "BARRIER-1").await,
        "barrier must reach designer, proving the cycle advanced past DEV-TO-PM"
    );
    assert!(
        !farm.capture_contains("designer", "DEV-TO-PM")
            && !farm.capture_contains("dev", "DEV-TO-PM"),
        "a dev->pm direct is WITHHELD from designer (not addressed) AND from dev (the sender never receives its own)"
    );

    post_message(
        &bpmux,
        json!({ "from": "dev", "text": "DEV-TO-PM-DESIGNER", "to": ["pm", "designer"] }),
    )
    .await;
    post_message(&bpmux, json!({ "from": "dev", "text": "BARRIER-2" })).await;
    assert!(
        wait_capture(&farm, "pm", "DEV-TO-PM-DESIGNER").await
            && wait_capture(&farm, "designer", "DEV-TO-PM-DESIGNER").await,
        "vice-versa GROUP: a message dev sends to:[pm,designer] reaches BOTH addressed terminals"
    );
    assert!(
        wait_capture(&farm, "pm", "BARRIER-2").await,
        "barrier must reach pm, proving the cycle advanced past dev's own send"
    );
    assert!(
        !farm.capture_contains("dev", "DEV-TO-PM-DESIGNER"),
        "the sender (dev) never receives its own group send"
    );
}
