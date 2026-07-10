pub fn short_name(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub fn bridge_agent_name(terminal_id: &str) -> String {
    let prefix = &terminal_id[..terminal_id.len().min(8)];
    format!("agent-{prefix}")
}

#[cfg(test)]
mod tests {
    use super::short_name;

    #[test]
    fn short_name_strips_tool_suffix() {
        assert_eq!(short_name("dev:claude-code"), "dev");
        assert_eq!(short_name("owner:human"), "owner");
        assert_eq!(short_name("bpmux"), "bpmux");
    }
}
