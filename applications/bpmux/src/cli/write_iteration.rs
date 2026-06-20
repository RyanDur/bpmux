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

pub(crate) fn run_iteration_remove_member(
    iteration: u64,
    story: u64,
    reason: &str,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    args.insert("story_number".to_string(), serde_json::json!(story));
    args.insert("reason".to_string(), serde_json::json!(reason));
    let role = crate::resolve_role(&None);
    let message = super::write::post_rpc_write(
        "remove_iteration_member",
        &role,
        &serde_json::Value::Object(args),
    )?;
    println!("✓ {message}");
    println!("  verify: bpmux board");
    Ok(())
}

pub(crate) fn run_iteration_move_member(iteration: u64, story: u64, to: u32) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    args.insert("story_number".to_string(), serde_json::json!(story));
    args.insert("position".to_string(), serde_json::json!(to));
    let role = crate::resolve_role(&None);
    let message = super::write::post_rpc_write(
        "move_iteration_member",
        &role,
        &serde_json::Value::Object(args),
    )?;
    println!("✓ {message}");
    println!("  verify: bpmux board");
    Ok(())
}

pub(crate) fn run_iteration_edit_theme(iteration: u64, theme: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    args.insert("theme".to_string(), serde_json::json!(theme));
    let role = crate::resolve_role(&None);
    let message = super::write::post_rpc_write(
        "edit_iteration_theme",
        &role,
        &serde_json::Value::Object(args),
    )?;
    println!("✓ {message}");
    println!("  verify: bpmux roadmap");
    Ok(())
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

pub(crate) fn run_epic_children(epic: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("epic_number".to_string(), serde_json::json!(epic));
    super::write::dispatch_write("list_epic_children", args)
}
