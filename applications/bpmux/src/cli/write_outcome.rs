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

pub(crate) fn run_outcome_retire(outcome: u64, reason: &str) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("outcome_number".to_string(), serde_json::json!(outcome));
    args.insert("reason".to_string(), serde_json::json!(reason));
    let role = crate::resolve_role(&None);
    let message =
        super::write::post_rpc_write("retire_outcome", &role, &serde_json::Value::Object(args))?;
    println!("✓ {message}");
    println!("  verify: bpmux roadmap");
    Ok(())
}

pub(crate) fn run_outcome_unmap(outcome: u64, iteration: u64) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("outcome_number".to_string(), serde_json::json!(outcome));
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    let role = crate::resolve_role(&None);
    let message = super::write::post_rpc_write(
        "unmap_outcome_iteration",
        &role,
        &serde_json::Value::Object(args),
    )?;
    println!("✓ {message}");
    println!("  verify: bpmux roadmap --outcome {outcome}");
    Ok(())
}

pub(crate) fn run_outcome_reorder(outcome: u64, iteration: u64, to: u32) -> anyhow::Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("outcome_number".to_string(), serde_json::json!(outcome));
    args.insert("iteration_number".to_string(), serde_json::json!(iteration));
    args.insert("position".to_string(), serde_json::json!(to));
    let role = crate::resolve_role(&None);
    let message = super::write::post_rpc_write(
        "reorder_outcome_iteration",
        &role,
        &serde_json::Value::Object(args),
    )?;
    println!("✓ {message}");
    println!("  verify: bpmux roadmap --outcome {outcome}");
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
