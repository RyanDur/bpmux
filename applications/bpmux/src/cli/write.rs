pub(crate) fn post_rpc_write(
    tool: &str,
    agent: &str,
    args: &serde_json::Value,
) -> anyhow::Result<String> {
    let port = crate::resolve_bpmux_port();
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve auth-token path (HOME unset?)"))?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| anyhow::anyhow!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, &format!("/api/rpc/{tool}"));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (status, text) = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let response = client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(crate::transport::rpc_routes::AGENT_HEADER, agent)
            .json(args)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Ok::<_, reqwest::Error>((status, text))
    })?;
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if status.is_success() {
        parsed
            .get("message")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "bpmux {tool}: success {status} but response missing 'message': {text}"
                )
            })
    } else {
        let reason = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or(text.as_str())
            .to_string();
        anyhow::bail!("bpmux {tool} rejected ({status}): {reason}")
    }
}

fn dispatch_write(
    tool: &str,
    args: serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let message = post_rpc_write(tool, &role, &serde_json::Value::Object(args))?;
    println!("{message}");
    Ok(())
}

pub(crate) fn run_pass(number: u64, acceptor: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("acceptor".to_string(), serde_json::json!(acceptor));
    args.insert("would_file_bug".to_string(), serde_json::json!(false));
    dispatch_write("pass_desk_check", args)
}

pub(crate) fn run_start(number: u64, paraphrase: &str, falsifier: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert(
        "pickup_paraphrase".to_string(),
        serde_json::json!(paraphrase),
    );
    args.insert(
        "falsifier_reframing".to_string(),
        serde_json::json!(falsifier),
    );
    dispatch_write("start_story", args)
}

pub(crate) fn run_deliver(number: u64, observation: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert(
        "verified_observation".to_string(),
        serde_json::json!(observation),
    );
    dispatch_write("deliver_story", args)
}

pub(crate) fn run_fail(number: u64, reason: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("reason".to_string(), serde_json::json!(reason));
    dispatch_write("fail_desk_check", args)
}

pub(crate) fn run_reject(number: u64, reason: &Option<String>) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    if let Some(r) = reason {
        args.insert("reason".to_string(), serde_json::json!(r));
    }
    dispatch_write("reject_story", args)
}

pub(crate) fn run_verdict(number: u64, verdict: &str, grounding: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("story_number".to_string(), serde_json::json!(number));
    args.insert("verdict".to_string(), serde_json::json!(verdict));
    args.insert(
        "substrate_grounding".to_string(),
        serde_json::json!(grounding),
    );
    dispatch_write("submit_verdict", args)
}

pub(crate) fn run_accept(
    number: u64,
    would_file_bug: bool,
    force: bool,
    bypass_reason: &Option<String>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert(
        "would_file_bug".to_string(),
        serde_json::json!(would_file_bug),
    );
    if force {
        args.insert("force".to_string(), serde_json::json!(true));
    }
    if let Some(r) = bypass_reason {
        args.insert("bypass_reason".to_string(), serde_json::json!(r));
    }
    dispatch_write("accept_story", args)
}

pub(crate) fn run_move(
    number: u64,
    target_state: &str,
    rationale: &str,
    position: Option<usize>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("target_state".to_string(), serde_json::json!(target_state));
    args.insert("rationale".to_string(), serde_json::json!(rationale));
    if let Some(p) = position {
        args.insert("position".to_string(), serde_json::json!(p));
    }
    dispatch_write("move_story", args)
}

pub(crate) fn run_label(
    number: u64,
    label: &str,
    operation: &Option<String>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("label".to_string(), serde_json::json!(label));
    if let Some(op) = operation {
        args.insert("operation".to_string(), serde_json::json!(op));
    }
    dispatch_write("label_story", args)
}

pub(crate) fn run_create(
    title: &str,
    user_job: &str,
    observation_falsifier: &str,
    body: &Option<String>,
    parent_epic: Option<u64>,
    acceptance_test_path: &Option<String>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("title".to_string(), serde_json::json!(title));
    args.insert("user_job".to_string(), serde_json::json!(user_job));
    args.insert(
        "observation_falsifier".to_string(),
        serde_json::json!(observation_falsifier),
    );
    if let Some(b) = body {
        args.insert("body".to_string(), serde_json::json!(b));
    }
    if let Some(e) = parent_epic {
        args.insert("parent_epic".to_string(), serde_json::json!(e));
    }
    if let Some(p) = acceptance_test_path {
        args.insert("acceptance_test_path".to_string(), serde_json::json!(p));
    }
    dispatch_write("create_story", args)
}

pub(crate) fn run_edit(
    number: u64,
    title: &Option<String>,
    body: &Option<String>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    if let Some(t) = title {
        args.insert("title".to_string(), serde_json::json!(t));
    }
    if let Some(b) = body {
        args.insert("body".to_string(), serde_json::json!(b));
    }
    dispatch_write("edit_story", args)
}

pub(crate) fn run_comment(number: u64, body: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("body".to_string(), serde_json::json!(body));
    dispatch_write("comment_on_story", args)
}

pub(crate) fn run_close(
    number: u64,
    reason: &str,
    disposition: &Option<String>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("reason".to_string(), serde_json::json!(reason));
    if let Some(d) = disposition {
        args.insert("disposition".to_string(), serde_json::json!(d));
    }
    dispatch_write("close_story", args)
}

pub(crate) fn run_iteration_create(theme: &str, position_after: Option<u64>) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("theme_name".to_string(), serde_json::json!(theme));
    if let Some(p) = position_after {
        args.insert("position_after".to_string(), serde_json::json!(p));
    }
    dispatch_write("create_iteration", args)
}

pub(crate) fn run_iteration_designate(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(number));
    dispatch_write("designate_current_iteration", args)
}

pub(crate) fn run_iteration_close_review(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(number));
    dispatch_write("close_pre_pickup_review", args)
}

pub(crate) fn run_iteration_dev_ok(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(number));
    dispatch_write("dev_ok_iteration", args)
}

pub(crate) fn run_outcome_create(
    title: &str,
    horizon: &str,
    body: Option<&str>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("title".to_string(), serde_json::json!(title));
    args.insert("horizon".to_string(), serde_json::json!(horizon));
    if let Some(b) = body {
        args.insert("body".to_string(), serde_json::json!(b));
    }
    dispatch_write("create_outcome", args)
}

pub(crate) fn run_epic_set_parent(number: u64, epic: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("epic_number".to_string(), serde_json::json!(epic));
    dispatch_write("set_parent_epic", args)
}

pub(crate) fn run_epic_unset_parent(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    dispatch_write("unset_parent_epic", args)
}

pub(crate) fn run_iteration_add(iteration: u64, story: u64, position: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    args.insert("story_number".to_string(), serde_json::json!(story));
    args.insert("position".to_string(), serde_json::json!(position));
    dispatch_write("add_story_to_iteration", args)
}

pub(crate) fn run_iteration_close(iteration: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    dispatch_write("close_iteration", args)
}

pub(crate) fn run_epic_children(epic: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("epic_number".to_string(), serde_json::json!(epic));
    dispatch_write("list_epic_children", args)
}

pub(crate) fn run_backlog() -> anyhow::Result<()> {
    dispatch_write("view_backlog", serde_json::Map::new())
}

pub(crate) fn run_comments(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    dispatch_write("view_comments", args)
}

pub(crate) fn run_timeline(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    dispatch_write("view_story_timeline", args)
}

pub(crate) fn run_verdicts(story: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("story_number".to_string(), serde_json::json!(story));
    dispatch_write("view_verdicts", args)
}

pub(crate) fn run_velocity() -> anyhow::Result<()> {
    dispatch_write("view_velocity", serde_json::Map::new())
}

pub(crate) fn run_story_commits(story: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(story));
    dispatch_write("view_story_commits", args)
}

pub(crate) fn run_commit_diff(sha: &str, max_bytes: Option<u64>) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("sha".to_string(), serde_json::json!(sha));
    if let Some(n) = max_bytes {
        args.insert("max_bytes".to_string(), serde_json::json!(n));
    }
    dispatch_write("view_commit_diff", args)
}
