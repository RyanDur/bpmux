use tokio::sync::Mutex;

use super::helpers::WsSink;

pub fn set_status_retired_error() -> String {
    serde_json::json!({
        "kind": "tool-retired",
        "retired_axis": "manual-status-mutation",
        "successor": "workflow-event-hooks",
        "message": "set_status is retired per #1404. Status is managed automatically by hooks per `feedback_bpmux_no_manual_status` canon — workflow-events (start_story / deliver_story / pass_desk_check / accept_story / reject_story / close_story) propagate state via hook-automation. Fire the workflow-event that matches your intent; do not call set_status manually."
    })
    .to_string()
}

pub async fn handle_set_status(
    _args: &serde_json::Map<String, serde_json::Value>,
    _ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    Err(set_status_retired_error())
}
