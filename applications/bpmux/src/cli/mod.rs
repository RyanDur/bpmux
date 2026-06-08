mod commands;
mod write;
pub(crate) use commands::{Command, RoomAction};
pub(crate) use write::{
    run_accept, run_deliver, run_fail, run_pass, run_reject, run_start, run_verdict,
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

fn http_get(url: &str) -> anyhow::Result<(reqwest::StatusCode, String)> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result: Result<(reqwest::StatusCode, String), reqwest::Error> = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let response = client.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;
        Ok((status, body))
    });
    Ok(result?)
}

fn fetch_board() -> anyhow::Result<serde_json::Value> {
    let port = crate::resolve_bpmux_port();
    let url = crate::transport::local_url::api_url(port, "/api/board");
    let (status, body) = http_get(&url)?;
    if !status.is_success() {
        anyhow::bail!("board fetch failed ({status}): {body}");
    }
    serde_json::from_str(&body).map_err(|e| anyhow::anyhow!("unparseable board response: {e}"))
}

fn board_stories(board: &serde_json::Value) -> Vec<serde_json::Value> {
    board["stories"].as_array().cloned().unwrap_or_default()
}

fn story_line(index: usize, story: &serde_json::Value) -> String {
    let number = story["number"].as_u64().unwrap_or(0);
    let title = story["title"].as_str().unwrap_or("");
    match story["acceptor"].as_str() {
        Some(acceptor) => format!("  {index}  #{number}  {title}  awaiting {acceptor}"),
        None => format!("  {index}  #{number}  {title}"),
    }
}

fn render_ready(ready: &[serde_json::Value]) -> String {
    let mut out = format!("ready ({}):\n", ready.len());
    for (i, story) in ready.iter().enumerate() {
        out.push_str(&story_line(i + 1, story));
        out.push('\n');
    }
    out
}

pub(crate) fn run_ready(json: bool, full: bool) -> anyhow::Result<()> {
    let board = fetch_board()?;
    if full {
        println!("{}", serde_json::to_string_pretty(&board)?);
        return Ok(());
    }
    let ready: Vec<serde_json::Value> = board_stories(&board)
        .into_iter()
        .filter(|s| s["state"].as_str() == Some("ready"))
        .collect();
    if json {
        println!("{}", serde_json::to_string_pretty(&ready)?);
        return Ok(());
    }
    print!("{}", render_ready(&ready));
    Ok(())
}

fn render_story(story: &serde_json::Value) -> String {
    let number = story["number"].as_u64().unwrap_or(0);
    let title = story["title"].as_str().unwrap_or("");
    let state = story["state"].as_str().unwrap_or("");
    let acceptor = story["acceptor"]
        .as_str()
        .map(|a| format!(" · awaiting {a}"))
        .unwrap_or_default();
    format!("#{number} {title} · {state}{acceptor}\n  use --full for the body\n")
}

pub(crate) fn run_story(number: u64, json: bool, full: bool) -> anyhow::Result<()> {
    let board = fetch_board()?;
    let Some(story) = board_stories(&board)
        .into_iter()
        .find(|s| s["number"].as_u64() == Some(number))
    else {
        anyhow::bail!("story #{number} not found");
    };
    if full {
        println!("{}", story["body"].as_str().unwrap_or(""));
        return Ok(());
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&story)?);
        return Ok(());
    }
    print!("{}", render_story(&story));
    Ok(())
}

fn render_board(stories: &[serde_json::Value]) -> String {
    let mut out = format!("board ({}):\n", stories.len());
    for (i, story) in stories.iter().enumerate() {
        let number = story["number"].as_u64().unwrap_or(0);
        let title = story["title"].as_str().unwrap_or("");
        let state = story["state"].as_str().unwrap_or("");
        out.push_str(&format!("  {}  #{number}  {title}  {state}\n", i + 1));
    }
    out
}

pub(crate) fn run_board(
    state: &Option<String>,
    limit: Option<usize>,
    json: bool,
    full: bool,
) -> anyhow::Result<()> {
    let board = fetch_board()?;
    if full {
        println!("{}", serde_json::to_string_pretty(&board)?);
        return Ok(());
    }
    let mut stories = board_stories(&board);
    if let Some(state_filter) = state {
        stories.retain(|s| s["state"].as_str() == Some(state_filter.as_str()));
    }
    if let Some(cap) = limit {
        stories.truncate(cap);
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&stories)?);
        return Ok(());
    }
    print!("{}", render_board(&stories));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn story(number: u64, title: &str, state: &str, acceptor: Option<&str>) -> serde_json::Value {
        serde_json::json!({
            "number": number,
            "title": title,
            "state": state,
            "acceptor": acceptor,
        })
    }

    #[test]
    fn render_ready_shows_count_header_and_one_line_per_ready_story() {
        let ready = vec![
            story(
                1656,
                "[Feature] Broadcast off Channels",
                "ready",
                Some("owner"),
            ),
            story(1683, "[Feature] Scoped board-read", "ready", Some("team")),
        ];
        let out = render_ready(&ready);
        assert!(out.starts_with("ready (2):\n"));
        assert!(out.contains("  1  #1656  [Feature] Broadcast off Channels  awaiting owner\n"));
        assert!(out.contains("  2  #1683  [Feature] Scoped board-read  awaiting team\n"));
    }

    #[test]
    fn render_ready_is_one_line_per_story_not_a_body_dump() {
        let ready = vec![story(1, "title", "ready", Some("owner"))];
        let out = render_ready(&ready);
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn story_line_without_acceptor_omits_the_awaiting_clause() {
        let s = story(42, "no acceptor", "ready", None);
        assert_eq!(story_line(1, &s), "  1  #42  no acceptor");
    }

    #[test]
    fn render_story_is_a_compact_summary_with_a_full_hint_not_the_body() {
        let s = story(1518, "[Bug] Reject-modal restyle", "ready", Some("owner"));
        let out = render_story(&s);
        assert!(out.contains("#1518 [Bug] Reject-modal restyle · ready · awaiting owner"));
        assert!(out.contains("use --full for the body"));
    }

    #[test]
    fn render_board_lists_one_line_per_story_with_state() {
        let stories = vec![
            story(1, "first", "ready", Some("owner")),
            story(2, "second", "in-progress", None),
        ];
        let out = render_board(&stories);
        assert!(out.starts_with("board (2):\n"));
        assert!(out.contains("  1  #1  first  ready\n"));
        assert!(out.contains("  2  #2  second  in-progress\n"));
    }
}
