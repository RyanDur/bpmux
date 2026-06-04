use tokio::sync::Mutex;

use super::super::tools::{WsSink, ws_send};
use crate::transport::protocol::ClientMessage;

fn role_id_for_purpose(purpose: &str) -> String {
    crate::workflow::current_workflow()
        .role_id_for_purpose(purpose)
        .map(str::to_string)
        .unwrap_or_else(|| purpose.to_string())
}

pub const SYNC_STATUS_UPDATE_MARKER: &str = "__BPMUX_SYNC_STATUS_UPDATE__";

pub(crate) fn build_board_invalidate_signal() -> ClientMessage {
    ClientMessage::BoardInvalidate
}

pub(crate) async fn broadcast_board_invalidate(ws_tx: &Mutex<Option<WsSink>>) {
    let msg = build_board_invalidate_signal();
    match serde_json::to_string(&msg) {
        Ok(json) => {
            if let Err(e) = ws_send(ws_tx, json).await {
                crate::session::diag::log_diag(&format!(
                    "bridge: board-invalidate signal failed: {e}"
                ));
            }
        }
        Err(e) => crate::session::diag::log_diag(&format!(
            "bridge: board-invalidate serialize failed: {e}"
        )),
    }
}

pub(crate) fn build_delivery_signals(number: u64, title: &str) -> Vec<ClientMessage> {
    let designer = role_id_for_purpose("designer");
    let pm = role_id_for_purpose("pm");
    vec![
        ClientMessage::Send {
            text: format!("#{number} ({title}) delivered."),
            to: Some(vec![designer.clone()]),
            addressee: Some(designer),
            message_type: None,
        },
        ClientMessage::Send {
            text: format!("#{number} ({title}) delivered."),
            to: Some(vec![pm.clone()]),
            addressee: Some(pm),
            message_type: None,
        },
    ]
}

pub(crate) fn build_start_signal(number: u64) -> ClientMessage {
    let pm = role_id_for_purpose("pm");
    ClientMessage::Send {
        text: format!("#{number} started."),
        to: Some(vec![pm.clone()]),
        addressee: Some(pm),
        message_type: None,
    }
}

pub(crate) fn build_accept_signal(number: u64, accepted_by: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} accepted by {accepted_by}."),
        to: None,
        addressee: Some(role_id_for_purpose("dev")),
        message_type: None,
    }
}

pub(crate) fn build_verdict_reprompt_signal(number: u64, role: &str) -> ClientMessage {
    let target = role_id_for_purpose(role);
    ClientMessage::Send {
        text: format!(
            "Your verdict for #{number} has expired (past the accept window). Re-cast it with submit_verdict so the accept can proceed."
        ),
        to: Some(vec![target.clone()]),
        addressee: Some(target),
        message_type: Some("system".to_string()),
    }
}

pub(crate) fn build_reject_signal(number: u64, reason: &str) -> ClientMessage {
    let dev = role_id_for_purpose("dev");
    ClientMessage::Send {
        text: format!("#{number} rejected: {reason}."),
        to: Some(vec![dev.clone()]),
        addressee: Some(dev),
        message_type: None,
    }
}

pub(crate) fn classify_for_pass_desk_check(
    number: u64,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> crate::board::domain::StoryType {
    refresh
        .and_then(|r| r.story(number))
        .map(|s| s.story_type)
        .unwrap_or(crate::board::domain::StoryType::Feature)
}

pub(crate) fn addressee_for_pass_desk_check(
    story_type: crate::board::domain::StoryType,
    labels: &[String],
) -> &'static str {
    let workflow = crate::workflow::current_workflow();
    let owner = || workflow.owner_role_id().unwrap_or("ryan");
    let team = || workflow.role_id_for_purpose("pm").unwrap_or("pm");

    let has_label = |needle: &str| labels.iter().any(|l| l.eq_ignore_ascii_case(needle));
    if has_label("awaiting team") {
        return team();
    }
    if has_label("awaiting owner") {
        return owner();
    }

    match story_type {
        crate::board::domain::StoryType::Feature
        | crate::board::domain::StoryType::Epic
        | crate::board::domain::StoryType::Iteration => owner(),
        crate::board::domain::StoryType::Bug | crate::board::domain::StoryType::Chore => team(),
    }
}

pub(crate) fn build_pass_desk_check_signal(number: u64, addressee: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} ready for acceptance. passed designer desk-check."),
        to: Some(vec![addressee.to_string()]),
        addressee: Some(addressee.to_string()),
        message_type: None,
    }
}

pub(crate) fn build_pass_desk_check_pm_present_signal(number: u64) -> ClientMessage {
    let pm = role_id_for_purpose("pm");
    ClientMessage::Send {
        text: format!("#{number} passed designer desk-check."),
        to: Some(vec![pm.clone()]),
        addressee: Some(pm),
        message_type: None,
    }
}

pub(crate) fn build_pass_desk_check_signals(
    number: u64,
    story_type: crate::board::domain::StoryType,
    labels: &[String],
) -> Vec<ClientMessage> {
    let primary = addressee_for_pass_desk_check(story_type, labels);
    let owner = crate::workflow::current_workflow()
        .owner_role_id()
        .unwrap_or("ryan");
    let mut signals = vec![build_pass_desk_check_signal(number, primary)];
    if primary == owner {
        signals.push(build_pass_desk_check_pm_present_signal(number));
    }
    signals
}

pub(crate) fn build_fail_desk_check_signal(number: u64, reason: &str) -> ClientMessage {
    let dev = role_id_for_purpose("dev");
    ClientMessage::Send {
        text: format!("#{number} failed desk-check: {reason}."),
        to: Some(vec![dev.clone()]),
        addressee: Some(dev),
        message_type: None,
    }
}

pub(crate) fn build_iteration_close_signal() -> ClientMessage {
    ClientMessage::Send {
        text: "[iteration-close] iteration closing — flush unsaved findings now via \
               Write / save_lesson / add_principle / doc edit. pre-close context is \
               still available."
            .to_string(),
        to: None,
        addressee: Some("all".to_string()),
        message_type: None,
    }
}
