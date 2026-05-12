use rmcp::model::Tool;

pub fn close_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "iteration_number": {
                "type": "integer",
                "description": "Issue number of the Iteration story (story_type=Iteration) to close (#1300, Epic #1189 slice 4). When provided, ceremony uses the canonical-via-GH path: closes the GH iteration-issue + promotes member-stories (queried via iteration_id) to ready via the canonical mutation chain — no priority.json icebox_unified manipulation. When omitted, ceremony uses the legacy markers-store path (preserves icebox_unified manipulation until slice 6 #1302 retires). Both paths preserve the #1258 r3 ready-empty guard, atomic-fire semantic, and IterationClose state-delta broadcast."
            }
        },
        "required": []
    });
    Tool::new(
        "close_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
