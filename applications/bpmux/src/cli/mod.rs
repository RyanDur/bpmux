mod actions;
mod bookmark;
mod commands;
mod principle;
mod read;
mod write;
pub(crate) use actions::{
    BookmarkAction, CiAction, EpicAction, IterationAction, LessonAction, PrincipleAction,
    RoomAction, ValueAction,
};
pub(crate) use bookmark::{run_bookmark_get, run_bookmark_save};
pub(crate) use commands::Command;
pub(crate) use principle::{
    run_identity_get, run_principle_add, run_principle_remove, run_principle_update, run_value_add,
};
pub(crate) use read::{
    run_board, run_capture, run_ci_jobs, run_ci_log, run_ci_rerun, run_ci_state, run_lesson_list,
    run_lesson_list_peer, run_lesson_save, run_ready, run_story,
};
pub(crate) use write::{
    run_accept, run_close, run_comment, run_create, run_deliver, run_edit, run_epic_set_parent,
    run_epic_unset_parent, run_fail, run_iteration_add, run_iteration_close, run_iteration_create,
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

pub(crate) fn dispatch(command: &Command) -> Option<anyhow::Result<()>> {
    match command {
        Command::EmitPrecommitPassed { sha, role } => Some(run_emit_precommit_passed(sha, role)),
        Command::WaitForEvent { after, timeout_ms } => Some(run_wait_for_event(after, *timeout_ms)),
        Command::Send {
            text,
            to,
            addressee,
        } => Some(run_send(text, to, addressee)),
        Command::Read { limit, full } => Some(run_read(*limit, *full)),
        Command::Help { json } => Some(run_help(*json)),
        Command::Ready { json, full } => Some(run_ready(*json, *full)),
        Command::Story { number, json, full } => Some(run_story(*number, *json, *full)),
        Command::Board {
            state,
            limit,
            json,
            full,
        } => Some(run_board(state, *limit, *json, *full)),
        Command::Capture { window } => Some(run_capture(*window)),
        Command::Pass { number, acceptor } => Some(run_pass(*number, acceptor)),
        Command::Start {
            number,
            paraphrase,
            falsifier,
        } => Some(run_start(*number, paraphrase, falsifier)),
        Command::Deliver {
            number,
            observation,
        } => Some(run_deliver(*number, observation)),
        Command::Fail { number, reason } => Some(run_fail(*number, reason)),
        Command::Reject { number, reason } => Some(run_reject(*number, reason)),
        Command::Verdict {
            number,
            verdict,
            grounding,
        } => Some(run_verdict(*number, verdict, grounding)),
        Command::Accept {
            number,
            would_file_bug,
            force,
            bypass_reason,
        } => Some(run_accept(*number, *would_file_bug, *force, bypass_reason)),
        Command::Move {
            number,
            target_state,
            rationale,
            position,
        } => Some(run_move(*number, target_state, rationale, *position)),
        Command::Label {
            number,
            label,
            operation,
        } => Some(run_label(*number, label, operation)),
        Command::Create {
            title,
            user_job,
            observation_falsifier,
            body,
            parent_epic,
            acceptance_test_path,
        } => Some(run_create(
            title,
            user_job,
            observation_falsifier,
            body,
            *parent_epic,
            acceptance_test_path,
        )),
        Command::Edit {
            number,
            title,
            body,
        } => Some(run_edit(*number, title, body)),
        Command::Comment { number, body } => Some(run_comment(*number, body)),
        Command::Close {
            number,
            reason,
            disposition,
        } => Some(run_close(*number, reason, disposition)),
        Command::Iteration { action } => Some(match action {
            IterationAction::Create {
                theme,
                position_after,
            } => run_iteration_create(theme, *position_after),
            IterationAction::Designate { number } => run_iteration_designate(*number),
            IterationAction::Add {
                iteration,
                story,
                position,
            } => run_iteration_add(*iteration, *story, *position),
            IterationAction::Close { iteration } => run_iteration_close(*iteration),
        }),
        Command::Epic { action } => Some(match action {
            EpicAction::SetParent { number, epic } => run_epic_set_parent(*number, *epic),
            EpicAction::UnsetParent { number } => run_epic_unset_parent(*number),
        }),
        Command::Lesson { action } => Some(match action {
            LessonAction::Save {
                topic,
                lesson,
                evidence,
            } => run_lesson_save(topic, lesson, evidence),
            LessonAction::List { topic, limit } => run_lesson_list(topic, *limit),
            LessonAction::ListPeer { peer } => run_lesson_list_peer(peer),
        }),
        Command::Ci { action } => Some(match action {
            CiAction::State => run_ci_state(),
            CiAction::Log { run } => run_ci_log(run),
            CiAction::Jobs { run } => run_ci_jobs(run),
            CiAction::Rerun { run } => run_ci_rerun(run),
        }),
        Command::Principle { action } => Some(match action {
            PrincipleAction::Add { value, principle } => run_principle_add(value, principle),
            PrincipleAction::Update { value, old, new } => run_principle_update(value, old, new),
            PrincipleAction::Remove { value, principle } => run_principle_remove(value, principle),
        }),
        Command::Value { action } => Some(match action {
            ValueAction::Add { name, description } => run_value_add(name, description),
        }),
        Command::Identity => Some(run_identity_get()),
        Command::Bookmark { action } => Some(match action {
            BookmarkAction::Save {
                current_task,
                next_step,
                notes,
                decisions,
                open_questions,
            } => run_bookmark_save(current_task, next_step, notes, decisions, open_questions),
            BookmarkAction::Get { role } => run_bookmark_get(role),
        }),
        Command::Bridge { .. }
        | Command::Recall { .. }
        | Command::Room { .. }
        | Command::Server => None,
    }
}
