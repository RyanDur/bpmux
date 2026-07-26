use super::http::{authed_get, http_get, http_get_with_query};

fn fetch_board() -> anyhow::Result<serde_json::Value> {
    let port = crate::bootstrap::resolve_bpmux_port();
    let url = local_url::api_url(port, "/api/board");
    let (status, body) = http_get(&url)?;
    if !status.is_success() {
        anyhow::bail!("board fetch failed ({status}): {body}");
    }
    serde_json::from_str(&body).map_err(|e| anyhow::anyhow!("unparseable board response: {e}"))
}

fn board_stories(board: &serde_json::Value) -> Vec<serde_json::Value> {
    board["stories"].as_array().cloned().unwrap_or_default()
}

pub fn story_line(index: usize, story: &serde_json::Value) -> String {
    let id = story["id"].as_u64().unwrap_or(0);
    let title = story["title"].as_str().unwrap_or("");
    match story["acceptor"].as_str() {
        Some(acceptor) => format!("  {index}  #{id}  {title}  awaiting {acceptor}"),
        None => format!("  {index}  #{id}  {title}"),
    }
}

pub fn render_ready(ready: &[serde_json::Value]) -> String {
    let mut out = format!("ready ({}):\n", ready.len());
    for (i, story) in ready.iter().enumerate() {
        out.push_str(&story_line(i + 1, story));
        out.push('\n');
    }
    out
}

pub fn run_ready(json: bool, full: bool) -> anyhow::Result<()> {
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

pub fn render_story(story: &serde_json::Value) -> String {
    let id = story["id"].as_u64().unwrap_or(0);
    let title = story["title"].as_str().unwrap_or("");
    let state = story["state"].as_str().unwrap_or("");
    let acceptor = story["acceptor"]
        .as_str()
        .map(|a| format!(" · awaiting {a}"))
        .unwrap_or_default();
    format!("#{id} {title} · {state}{acceptor}\n  use --full for the body\n")
}

pub fn run_story(id: u64, json: bool, full: bool) -> anyhow::Result<()> {
    let board = fetch_board()?;
    let Some(story) = board_stories(&board)
        .into_iter()
        .find(|s| s["id"].as_u64() == Some(id))
    else {
        anyhow::bail!("story #{id} not found");
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

pub fn render_board(stories: &[serde_json::Value]) -> String {
    let mut out = format!("board ({}):\n", stories.len());
    for (i, story) in stories.iter().enumerate() {
        let id = story["id"].as_u64().unwrap_or(0);
        let title = story["title"].as_str().unwrap_or("");
        let state = story["state"].as_str().unwrap_or("");
        match story["acceptor"].as_str() {
            Some(acceptor) => out.push_str(&format!(
                "  {}  #{id}  {title}  {state}  awaiting {acceptor}\n",
                i + 1
            )),
            None => out.push_str(&format!("  {}  #{id}  {title}  {state}\n", i + 1)),
        }
    }
    out
}

pub fn run_board(
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

pub fn run_capture(window: bool) -> anyhow::Result<()> {
    if !window {
        anyhow::bail!(
            "bpmux capture requires --window — the Electron renderer window is the only live capture surface; the TUI screen-capture was retired in #1166"
        );
    }
    let role = crate::bootstrap::resolve_role(&None);
    let port = crate::bootstrap::resolve_bpmux_port();
    let token_path = auth::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve auth-token path (HOME unset?)"))?;
    let token = auth::read(&token_path)
        .map_err(|e| anyhow::anyhow!("read auth-token from {}: {e}", token_path.display()))?;
    let url = local_url::api_url(port, &format!("/api/electron-capture?role={role}"));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (status, body) = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(15_000))
            .build()?;
        let response = client
            .get(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Ok::<_, reqwest::Error>((status, body))
    })?;
    if !status.is_success() {
        anyhow::bail!("bpmux capture --window failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}

pub fn run_sync_status() -> anyhow::Result<()> {
    let (status, body) = authed_get("/api/sync-status")?;
    if !status.is_success() {
        anyhow::bail!("sync-status failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}

pub fn run_search(
    query: &str,
    sender: &Option<String>,
    addressee: &Option<String>,
) -> anyhow::Result<()> {
    let mut params: Vec<(&str, &str)> = vec![("query", query)];
    if let Some(s) = sender {
        params.push(("sender", s));
    }
    if let Some(a) = addressee {
        params.push(("addressee", a));
    }
    let (status, body) = http_get_with_query("/api/search", &params)?;
    if !status.is_success() {
        anyhow::bail!("search failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}
