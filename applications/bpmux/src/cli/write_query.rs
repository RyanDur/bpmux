pub(crate) fn run_backlog() -> anyhow::Result<()> {
    super::write::dispatch_write("view_backlog", serde_json::Map::new())
}

pub(crate) fn run_comments(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    super::write::dispatch_write("view_comments", args)
}

pub(crate) fn run_timeline(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    super::write::dispatch_write("view_story_timeline", args)
}

pub(crate) fn run_verdicts(story: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("story_number".to_string(), serde_json::json!(story));
    super::write::dispatch_write("view_verdicts", args)
}

pub(crate) fn run_velocity() -> anyhow::Result<()> {
    super::write::dispatch_write("view_velocity", serde_json::Map::new())
}

pub(crate) fn run_story_commits(story: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(story));
    super::write::dispatch_write("view_story_commits", args)
}

pub(crate) fn run_commit_diff(sha: &str, max_bytes: Option<u64>) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("sha".to_string(), serde_json::json!(sha));
    if let Some(n) = max_bytes {
        args.insert("max_bytes".to_string(), serde_json::json!(n));
    }
    super::write::dispatch_write("view_commit_diff", args)
}
