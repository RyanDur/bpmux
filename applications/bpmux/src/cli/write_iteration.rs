pub(crate) fn run_iteration_create(theme: &str, position_after: Option<u64>) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("theme_name".to_string(), serde_json::json!(theme));
    if let Some(p) = position_after {
        args.insert("position_after".to_string(), serde_json::json!(p));
    }
    super::write::dispatch_write("create_iteration", args)
}

pub(crate) fn run_iteration_designate(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(number));
    super::write::dispatch_write("designate_current_iteration", args)
}

pub(crate) fn run_iteration_close_review(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(number));
    super::write::dispatch_write("close_pre_pickup_review", args)
}

pub(crate) fn run_iteration_dev_ok(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(number));
    super::write::dispatch_write("dev_ok_iteration", args)
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
    super::write::dispatch_write("create_outcome", args)
}

pub(crate) fn run_outcome_add(outcome: u64, iteration: u64, position: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("outcome_number".to_string(), serde_json::json!(outcome));
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    args.insert("position".to_string(), serde_json::json!(position));
    super::write::dispatch_write("map_iteration_to_outcome", args)
}

pub(crate) fn run_outcome_edit(
    outcome: u64,
    title: Option<&str>,
    body: Option<&str>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("outcome_number".to_string(), serde_json::json!(outcome));
    if let Some(t) = title {
        args.insert("title".to_string(), serde_json::json!(t));
    }
    if let Some(b) = body {
        args.insert("body".to_string(), serde_json::json!(b));
    }
    let role = crate::resolve_role(&None);
    let message =
        super::write::post_rpc_write("edit_outcome", &role, &serde_json::Value::Object(args))?;
    println!("✓ {message}");
    if title.is_some() {
        println!("  verify: bpmux roadmap");
    }
    if body.is_some() {
        println!("  verify: bpmux story {outcome}");
    }
    Ok(())
}

pub(crate) fn run_outcome_reprioritize(
    outcome: u64,
    to: Option<&str>,
    position: Option<u32>,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("outcome_number".to_string(), serde_json::json!(outcome));
    if let Some(band) = to {
        args.insert("to".to_string(), serde_json::json!(band));
    }
    if let Some(p) = position {
        args.insert("position".to_string(), serde_json::json!(p));
    }
    let role = crate::resolve_role(&None);
    let message = super::write::post_rpc_write(
        "reprioritize_outcome",
        &role,
        &serde_json::Value::Object(args),
    )?;
    println!("✓ {message}");
    println!("  verify: bpmux roadmap");
    Ok(())
}

pub(crate) fn run_epic_set_parent(number: u64, epic: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("epic_number".to_string(), serde_json::json!(epic));
    super::write::dispatch_write("set_parent_epic", args)
}

pub(crate) fn run_epic_unset_parent(number: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    super::write::dispatch_write("unset_parent_epic", args)
}

pub(crate) fn run_iteration_add(iteration: u64, story: u64, position: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    args.insert("story_number".to_string(), serde_json::json!(story));
    args.insert("position".to_string(), serde_json::json!(position));
    super::write::dispatch_write("add_story_to_iteration", args)
}

pub(crate) fn run_iteration_close(iteration: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    super::write::dispatch_write("close_iteration", args)
}

pub(crate) fn run_epic_children(epic: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("epic_number".to_string(), serde_json::json!(epic));
    super::write::dispatch_write("list_epic_children", args)
}
