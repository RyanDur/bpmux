pub(crate) fn run_iteration_create(name: &str, position_after: Option<u64>) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("name".to_string(), serde_json::json!(name));
    if let Some(p) = position_after {
        args.insert("position_after".to_string(), serde_json::json!(p));
    }
    super::write::dispatch_write("create_iteration", args)
}

pub(crate) fn run_iteration_designate(iteration_id: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_id".to_string(), serde_json::json!(iteration_id));
    super::write::dispatch_write("designate_current_iteration", args)
}

pub(crate) fn run_iteration_close_review(iteration_id: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_id".to_string(), serde_json::json!(iteration_id));
    super::write::dispatch_write("close_pre_pickup_review", args)
}

pub(crate) fn run_iteration_dev_ok(iteration_id: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_id".to_string(), serde_json::json!(iteration_id));
    super::write::dispatch_write("dev_ok_iteration", args)
}

pub(crate) fn run_iteration_remove_member(
    iteration: u64,
    story: u64,
    reason: &str,
) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_id".to_string(), serde_json::json!(iteration));
    args.insert("story_id".to_string(), serde_json::json!(story));
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
    args.insert("iteration_id".to_string(), serde_json::json!(iteration));
    args.insert("story_id".to_string(), serde_json::json!(story));
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

pub(crate) fn run_iteration_rename(iteration: u64, name: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_id".to_string(), serde_json::json!(iteration));
    args.insert("name".to_string(), serde_json::json!(name));
    let role = crate::resolve_role(&None);
    let message =
        super::write::post_rpc_write("rename_iteration", &role, &serde_json::Value::Object(args))?;
    println!("✓ {message}");
    println!("  verify: bpmux roadmap");
    Ok(())
}

pub(crate) fn run_iteration_add(iteration: u64, story: u64, position: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_id".to_string(), serde_json::json!(iteration));
    args.insert("story_id".to_string(), serde_json::json!(story));
    args.insert("position".to_string(), serde_json::json!(position));
    super::write::dispatch_write("add_story_to_iteration", args)
}

pub(crate) fn run_iteration_close(iteration: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("iteration_id".to_string(), serde_json::json!(iteration));
    super::write::dispatch_write("close_iteration", args)
}

pub(crate) fn run_epic_set_parent(id: u64, epic: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("id".to_string(), serde_json::json!(id));
    args.insert("epic_id".to_string(), serde_json::json!(epic));
    super::write::dispatch_write("set_parent_epic", args)
}

pub(crate) fn run_epic_unset_parent(id: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("id".to_string(), serde_json::json!(id));
    super::write::dispatch_write("unset_parent_epic", args)
}

pub(crate) fn run_epic_children(epic: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("epic_id".to_string(), serde_json::json!(epic));
    super::write::dispatch_write("list_epic_children", args)
}
