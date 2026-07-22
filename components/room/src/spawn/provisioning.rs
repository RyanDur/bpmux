use super::osc7::{Osc7Integration, osc7_shell_integration};

pub struct BridgeWiring {
    pub binary: String,
    pub port: u16,
    pub agent_name: String,
}

pub fn terminal_provisioning(
    command: &str,
    bridge: Option<&BridgeWiring>,
    owner_zdotdir: &str,
) -> std::io::Result<Osc7Integration> {
    let integration = osc7_shell_integration(command, owner_zdotdir)?;
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

pub(super) fn bridge_mcp_config(bridge: &BridgeWiring) -> String {
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
