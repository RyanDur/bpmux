pub(crate) fn run_pass(number: u64, acceptor: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("acceptor".to_string(), serde_json::json!(acceptor));
    args.insert("would_file_bug".to_string(), serde_json::json!(false));
    super::write::dispatch_write("pass_desk_check", args)
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
    super::write::dispatch_write("start_story", args)
}

pub(crate) fn run_deliver(number: u64, observation: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert(
        "verified_observation".to_string(),
        serde_json::json!(observation),
    );
    super::write::dispatch_write("deliver_story", args)
}

pub(crate) fn run_fail(number: u64, reason: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("reason".to_string(), serde_json::json!(reason));
    super::write::dispatch_write("fail_desk_check", args)
}

pub(crate) fn run_reject(number: u64, reason: &Option<String>) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    if let Some(r) = reason {
        args.insert("reason".to_string(), serde_json::json!(r));
    }
    super::write::dispatch_write("reject_story", args)
}

pub(crate) fn run_verdict(number: u64, verdict: &str, grounding: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("story_number".to_string(), serde_json::json!(number));
    args.insert("verdict".to_string(), serde_json::json!(verdict));
    args.insert(
        "substrate_grounding".to_string(),
        serde_json::json!(grounding),
    );
    super::write::dispatch_write("submit_verdict", args)
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
    super::write::dispatch_write("accept_story", args)
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
    super::write::dispatch_write("move_story", args)
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
    super::write::dispatch_write("label_story", args)
}

pub(crate) fn run_create(
    title: &str,
    user_job: &str,
    observation_falsifier: &str,
    body: &Option<String>,
    parent_epic: Option<u64>,
    acceptance_test_path: &Option<String>,
    acceptor: &Option<String>,
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
    if let Some(a) = acceptor {
        args.insert("acceptor".to_string(), serde_json::json!(a));
    }
    super::write::dispatch_write("create_story", args)
}

pub(crate) fn run_edit(
    number: u64,
    title: &Option<String>,
    body: &Option<String>,
    acceptor: &Option<String>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    if let Some(t) = title {
        args.insert("title".to_string(), serde_json::json!(t));
    }
    if let Some(b) = body {
        args.insert("body".to_string(), serde_json::json!(b));
    }
    if let Some(a) = acceptor {
        args.insert("acceptor".to_string(), serde_json::json!(a));
    }
    super::write::dispatch_write("edit_story", args)
}

pub(crate) fn run_comment(number: u64, body: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("body".to_string(), serde_json::json!(body));
    super::write::dispatch_write("comment_on_story", args)
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
    super::write::dispatch_write("close_story", args)
}
