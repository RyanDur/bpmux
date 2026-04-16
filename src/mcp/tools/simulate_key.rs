use rmcp::model::Tool;

pub fn simulate_key_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "key": {
                "type": "string",
                "description": "Key to press: single character, or 'enter', 'escape', 'tab', 'backspace', 'up', 'down', 'left', 'right'"
            },
            "modifiers": {
                "type": "array",
                "items": {
                    "type": "string",
                    "enum": ["ctrl", "control", "alt", "shift", "super"]
                },
                "description": "Optional modifier keys to apply (case-insensitive). Combine as an array: ['alt'], ['ctrl','shift'], []. Unknown names are rejected with a 400."
            }
        },
        "required": ["key"]
    });
    Tool::new(
        "simulate_key",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
