pub fn run_backlog() -> anyhow::Result<()> {
    super::write::dispatch_write("view_backlog", serde_json::Map::new())
}

pub fn run_comments(id: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("id".to_string(), serde_json::json!(id));
    super::write::dispatch_write("view_comments", args)
}

pub fn run_timeline(id: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("id".to_string(), serde_json::json!(id));
    super::write::dispatch_write("view_story_timeline", args)
}

pub fn run_verdicts(story: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("story_id".to_string(), serde_json::json!(story));
    super::write::dispatch_write("view_verdicts", args)
}

pub fn run_velocity() -> anyhow::Result<()> {
    super::write::dispatch_write("view_velocity", serde_json::Map::new())
}

pub fn run_story_commits(story: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("id".to_string(), serde_json::json!(story));
    super::write::dispatch_write("view_story_commits", args)
}

pub fn run_commit_diff(sha: &str, max_bytes: Option<u64>) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("sha".to_string(), serde_json::json!(sha));
    if let Some(n) = max_bytes {
        args.insert("max_bytes".to_string(), serde_json::json!(n));
    }
    super::write::dispatch_write("view_commit_diff", args)
}
