mod commands;
mod read;
mod write;
pub(crate) use commands::{Command, EpicAction, IterationAction, RoomAction};
pub(crate) use read::{run_board, run_capture, run_ready, run_story};
pub(crate) use write::{
    run_accept, run_close, run_comment, run_create, run_deliver, run_edit, run_epic_set_parent,
    run_epic_unset_parent, run_fail, run_iteration_add, run_iteration_create,
    run_iteration_designate, run_label, run_move, run_pass, run_reject, run_start, run_verdict,
};

const WAIT_CLIENT_GRACE_MS: u64 = 5_000;

pub(crate) fn emit_recall_activity(role: &str, count: usize) {
    let body = crate::session::recall::build_recall_activity_payload(role, count);
    post_activity(&body);
}

pub(crate) fn run_wait_for_event(after: &Option<String>, timeout_ms: u64) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let port = crate::resolve_bpmux_port();
    let mut url = crate::transport::local_url::api_url(port, "/api/wait-for-event");
    url.push_str(&format!("?as={role}&timeout_ms={timeout_ms}"));
    if let Some(cursor) = after {
        url.push_str(&format!("&after={cursor}"));
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let body = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(
                timeout_ms + WAIT_CLIENT_GRACE_MS,
            ))
            .build()?;
        client.get(&url).send().await?.text().await
    })?;
    println!("{body}");
    Ok(())
}

pub(crate) fn run_send(
    text: &str,
    to: &[String],
    addressee: &Option<String>,
) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let mut payload = serde_json::Map::new();
    payload.insert(
        "from".to_string(),
        serde_json::json!(format!("{role}:claude-code")),
    );
    payload.insert("text".to_string(), serde_json::json!(text));
    if !to.is_empty() {
        payload.insert("to".to_string(), serde_json::json!(to));
    }
    if let Some(target) = addressee {
        payload.insert("addressee".to_string(), serde_json::json!(target));
    }
    let body = serde_json::Value::Object(payload);
    let port = crate::resolve_bpmux_port();
    let url = crate::transport::local_url::api_url(port, "/api/messages");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (status, reason) = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let response = client.post(&url).json(&body).send().await?;
        let status = response.status();
        let reason = response.text().await.unwrap_or_default();
        Ok::<_, reqwest::Error>((status, reason))
    })?;
    if !status.is_success() {
        anyhow::bail!("send rejected ({status}): {reason}");
    }
    let destination = if to.is_empty() {
        "broadcast".to_string()
    } else {
        format!("[{}]", to.join(", "))
    };
    println!("✓ sent → {destination}");
    Ok(())
}

pub(crate) fn run_read(limit: usize, full: bool) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let port = crate::resolve_bpmux_port();
    let mut url = crate::transport::local_url::api_url(port, "/api/messages");
    url.push_str(&format!("?limit={limit}&chat_only=true&as={role}"));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let body = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        client.get(&url).send().await?.text().await
    })?;
    if full {
        println!("{body}");
        return Ok(());
    }
    let messages: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap_or_default();
    for message in messages.iter().rev() {
        let sender = message["from"]
            .as_str()
            .unwrap_or("?")
            .split(':')
            .next()
            .unwrap_or("?");
        let text = message["text"].as_str().unwrap_or("");
        println!("{sender}: {text}");
    }
    Ok(())
}

pub(crate) fn run_help(json: bool) -> anyhow::Result<()> {
    let command = <crate::Cli as clap::CommandFactory>::command();
    if json {
        let verbs: Vec<serde_json::Value> = command
            .get_subcommands()
            .map(|sub| {
                serde_json::json!({
                    "verb": sub.get_name(),
                    "about": sub.get_about().map(|about| about.to_string()).unwrap_or_default(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "verbs": verbs }))?
        );
    } else {
        for sub in command.get_subcommands() {
            let about = sub
                .get_about()
                .map(|about| about.to_string())
                .unwrap_or_default();
            println!("{:20} {about}", sub.get_name());
        }
    }
    Ok(())
}

pub(crate) fn run_emit_precommit_passed(
    sha: &str,
    role_arg: &Option<String>,
) -> anyhow::Result<()> {
    let role = crate::resolve_role(role_arg);
    let body = crate::session::precommit::build_precommit_activity_payload(&role, sha);
    post_activity(&body);
    Ok(())
}

fn post_activity(body: &serde_json::Value) {
    let port = crate::resolve_bpmux_port();
    let url = crate::transport::local_url::api_url(port, "/api/messages");
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build();
        let Ok(client) = client else { return };
        let _ = client.post(&url).json(&body).send().await;
    });
}
