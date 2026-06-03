use super::osc7::{Osc7Integration, osc7_shell_integration};

pub struct BridgeWiring {
    pub binary: String,
    pub port: u16,
    pub agent_name: String,
}

pub fn terminal_provisioning(
    command: &str,
    bridge: Option<&BridgeWiring>,
) -> std::io::Result<Osc7Integration> {
    let integration = osc7_shell_integration(command)?;
    if let (Some(bridge), Some(dir)) = (bridge, integration.zdotdir.as_ref())
        && let Some(rcfile) = provisioned_rcfile(command, dir.path())
    {
        let bridge_json = dir.path().join("bpmux-bridge.json");
        std::fs::write(&bridge_json, bridge_mcp_config(bridge))?;
        append_agent_wrapper(&rcfile, "claude", &bridge_json)?;
    }
    Ok(integration)
}

fn provisioned_rcfile(command: &str, tempdir: &std::path::Path) -> Option<std::path::PathBuf> {
    let family = std::path::Path::new(command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    match family {
        "bash" => Some(tempdir.join("bpmux-bashrc")),
        "zsh" => Some(tempdir.join(".zshrc")),
        _ => None,
    }
}

fn bridge_mcp_config(bridge: &BridgeWiring) -> String {
    serde_json::json!({
        "mcpServers": {
            "bpmux-bridge": {
                "type": "stdio",
                "command": bridge.binary,
                "args": [
                    "bridge",
                    "--port",
                    bridge.port.to_string(),
                    "--name",
                    bridge.agent_name,
                ],
            }
        }
    })
    .to_string()
}

fn append_agent_wrapper(
    rcfile: &std::path::Path,
    agent: &str,
    config: &std::path::Path,
) -> std::io::Result<()> {
    use std::io::Write;
    let config = config.to_string_lossy();
    let block = format!("{agent}() {{ command {agent} --mcp-config=\"{config}\" \"$@\"; }}\n");
    let mut file = std::fs::OpenOptions::new().append(true).open(rcfile)?;
    file.write_all(block.as_bytes())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::room::spawn::{LivePty, spawn_pty_retained};

    #[test]
    fn provisioned_bash_wraps_claude_with_mcp_config_and_preserves_user_args() {
        use std::io::Read;
        use std::os::unix::fs::PermissionsExt;
        let bash = "/bin/bash";
        if !std::path::Path::new(bash).exists() {
            return;
        }
        let home = tempfile::tempdir().unwrap();
        let bindir = home.path().join("bin");
        std::fs::create_dir_all(&bindir).unwrap();
        let stub = bindir.join("claude");
        std::fs::write(&stub, "#!/bin/sh\nprintf 'STUBCLAUDE[%s]\\n' \"$*\"\n").unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

        let bridge = BridgeWiring {
            binary: "/opt/bin/bpmux".to_string(),
            port: 4117,
            agent_name: "agent-term1".to_string(),
        };
        let integration = terminal_provisioning(bash, Some(&bridge)).unwrap();

        let home_str = home.path().to_string_lossy().to_string();
        let bindir_str = bindir.to_string_lossy().to_string();
        let base_path = std::env::var("PATH").unwrap_or_default();
        let path_str = format!("{bindir_str}:{base_path}");
        let mut env: Vec<(&str, &str)> = vec![
            ("TERM", "xterm-256color"),
            ("HOME", home_str.as_str()),
            ("PATH", path_str.as_str()),
        ];
        for (key, value) in &integration.env {
            env.push((key.as_str(), value.as_str()));
        }
        let mut live = spawn_pty_retained(bash, &integration.args, None, &env).unwrap();
        if let Some(dir) = integration.zdotdir {
            live.attach_tempdir(dir);
        }
        live.write_input(b"claude hello world\n").unwrap();
        let reader = live.reader().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let reader_handle = std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let mut collected = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(chunk) => {
                    collected.push_str(&String::from_utf8_lossy(&chunk));
                    if collected.contains("STUBCLAUDE[") && collected.contains("hello world") {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        live.close_and_reap();
        drop(rx);
        let _ = reader_handle.join();
        assert!(
            collected.contains("--mcp-config="),
            "the provisioned shell must wrap claude to invoke the real binary with --mcp-config= (=form so the variadic flag binds one value, not the user prompt); got {collected:?}"
        );
        assert!(
            collected.contains("hello world"),
            "the wrapper must PRESERVE the user's args end-to-end (claude hello world reaches the real claude as the prompt, not swallowed into the variadic config flag); got {collected:?}"
        );
    }

    fn spawn_provisioned_shell_with_stub(
        shell: &str,
        bridge: Option<&BridgeWiring>,
    ) -> (LivePty, tempfile::TempDir) {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let bindir = home.path().join("bin");
        std::fs::create_dir_all(&bindir).unwrap();
        let stub = bindir.join("claude");
        std::fs::write(&stub, "#!/bin/sh\nprintf 'STUBCLAUDE[%s]\\n' \"$*\"\n").unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let integration = terminal_provisioning(shell, bridge).unwrap();
        let home_str = home.path().to_string_lossy().to_string();
        let base_path = std::env::var("PATH").unwrap_or_default();
        let path_str = format!("{}:{base_path}", bindir.to_string_lossy());
        let mut env: Vec<(&str, &str)> = vec![
            ("TERM", "xterm-256color"),
            ("HOME", home_str.as_str()),
            ("PATH", path_str.as_str()),
        ];
        for (key, value) in &integration.env {
            env.push((key.as_str(), value.as_str()));
        }
        let mut live = spawn_pty_retained(shell, &integration.args, None, &env).unwrap();
        if let Some(dir) = integration.zdotdir {
            live.attach_tempdir(dir);
        }
        (live, home)
    }

    fn collect_matching(
        live: LivePty,
        input: &[u8],
        secs: u64,
        mut done: impl FnMut(&str) -> bool,
    ) -> String {
        use std::io::Read;
        live.write_input(input).unwrap();
        let reader = live.reader().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let handle = std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let mut collected = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
        while std::time::Instant::now() < deadline {
            match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(chunk) => {
                    collected.push_str(&String::from_utf8_lossy(&chunk));
                    if done(&collected) {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        live.close_and_reap();
        drop(rx);
        let _ = handle.join();
        collected
    }

    fn collect_until(live: LivePty, input: &[u8], until: &str, secs: u64) -> String {
        let until = until.to_string();
        collect_matching(live, input, secs, move |seen| seen.contains(&until))
    }

    #[test]
    fn unprovisioned_bash_gives_claude_no_bridge() {
        if !std::path::Path::new("/bin/bash").exists() {
            return;
        }
        let (live, _home) = spawn_provisioned_shell_with_stub("/bin/bash", None);
        let collected = collect_until(live, b"claude probe\n", "STUBCLAUDE[", 8);
        assert!(
            collected.contains("STUBCLAUDE["),
            "the stub claude must run so the probe is meaningful; got {collected:?}"
        );
        assert!(
            !collected.contains("--mcp-config="),
            "an UNPROVISIONED shell (bridge=None) must give claude NO bridge — proves the wrapper did the work, not an ambient config (honest pre-state discriminator); got {collected:?}"
        );
    }

    #[test]
    fn provisioned_bash_leaves_non_claude_commands_unaffected() {
        if !std::path::Path::new("/bin/bash").exists() {
            return;
        }
        let bridge = BridgeWiring {
            binary: "/opt/bin/bpmux".to_string(),
            port: 4117,
            agent_name: "agent-term1".to_string(),
        };
        let (live, _home) = spawn_provisioned_shell_with_stub("/bin/bash", Some(&bridge));
        let collected = collect_until(live, b"echo PLAINMARKER_4731\n", "PLAINMARKER_4731", 8);
        assert!(
            collected.contains("PLAINMARKER_4731"),
            "the provisioned terminal must stay a generic shell — a NON-claude command runs unaffected (the agent-wrapper is inert unless claude is launched); got {collected:?}"
        );
    }

    #[test]
    fn bridge_mcp_config_declares_the_session_bridge_with_a_legible_name() {
        let bridge = BridgeWiring {
            binary: "/opt/bin/bpmux".to_string(),
            port: 4117,
            agent_name: "agent-term1".to_string(),
        };
        let cfg: serde_json::Value = serde_json::from_str(&bridge_mcp_config(&bridge)).unwrap();
        let server = &cfg["mcpServers"]["bpmux-bridge"];
        assert_eq!(
            server["command"], "/opt/bin/bpmux",
            "the injected config must point at THIS running bpmux binary so the agent reconnects to the live session"
        );
        assert_eq!(
            server["args"],
            serde_json::json!(["bridge", "--port", "4117", "--name", "agent-term1"]),
            "the bridge args carry the session port + a legible per-terminal --name (else identity collides across N terminals)"
        );
        assert_eq!(server["type"], "stdio");
    }

    #[test]
    fn provisioned_zsh_wraps_claude_with_mcp_config_and_preserves_user_args() {
        if !std::path::Path::new("/bin/zsh").exists() {
            return;
        }
        let bridge = BridgeWiring {
            binary: "/opt/bin/bpmux".to_string(),
            port: 4117,
            agent_name: "agent-term1".to_string(),
        };
        let (live, _home) = spawn_provisioned_shell_with_stub("/bin/zsh", Some(&bridge));
        let collected = collect_matching(live, b"claude hello world\n", 8, |seen| {
            seen.contains("STUBCLAUDE[") && seen.contains("hello world")
        });
        assert!(
            collected.contains("--mcp-config="),
            "the provisioned ZSH (the owner's actual shell, sourced via ZDOTDIR/.zshrc — a DIFFERENT path than bash's --rcfile) must fire the claude wrapper with --mcp-config=; measured not reasoned because the owner walks zsh; got {collected:?}"
        );
        assert!(
            collected.contains("hello world"),
            "the zsh wrapper must preserve the user's args end-to-end; got {collected:?}"
        );
    }
}
