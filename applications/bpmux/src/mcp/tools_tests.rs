use super::tools::*;

#[test]
fn send_message_description_warns_against_name_prefix() {
    let desc =
        "Send a message. Do not prefix your name — the session adds the sender automatically.";
    let tool = send_message_tool(desc);
    let tool_desc = tool.description.unwrap_or_default();
    assert!(
        tool_desc.contains("Do not prefix your name"),
        "send_message description should warn against self-prefixing, got: {tool_desc}"
    );
}
