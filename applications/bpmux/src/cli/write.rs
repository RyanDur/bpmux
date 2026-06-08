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
