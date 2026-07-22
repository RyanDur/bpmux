use super::http::http_get_with_query;

pub fn run_roadmap(
    outcome: Option<u64>,
    state: Option<String>,
    order: Option<String>,
    limit: Option<usize>,
) -> anyhow::Result<()> {
    validate_state(state.as_deref())?;
    validate_order(order.as_deref())?;
    let mut owned: Vec<(&str, String)> = Vec::new();
    if let Some(number) = outcome {
        owned.push(("outcome", number.to_string()));
    }
    if let Some(value) = &state {
        owned.push(("state", value.clone()));
    }
    if let Some(value) = &order {
        owned.push(("order", value.clone()));
    }
    if let Some(value) = limit {
        owned.push(("limit", value.to_string()));
    }
    let params: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let (status, body) = http_get_with_query("/api/roadmap", &params)?;
    if !status.is_success() {
        anyhow::bail!("roadmap fetch failed ({status}): {body}");
    }
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("unparseable roadmap response: {e}"))?;
    let out = if outcome.is_some() {
        render_drill(&value)
    } else {
        view::roadmap_render::render_overview_layer(&value)
    };
    print!("{out}");
    Ok(())
}

fn validate_state(state: Option<&str>) -> anyhow::Result<()> {
    if let Some(value) = state
        && product_management::roadmap::selectors::OutcomeState::from_cli(value).is_none()
    {
        anyhow::bail!(
            "invalid --state '{value}': must be {}",
            product_management::roadmap::selectors::OutcomeState::CLI_VALUES
        );
    }
    Ok(())
}

fn validate_order(order: Option<&str>) -> anyhow::Result<()> {
    if let Some(value) = order
        && product_management::roadmap::filter::RoadmapOrder::from_cli(value).is_none()
    {
        anyhow::bail!(
            "invalid --order '{value}': must be {}",
            product_management::roadmap::filter::RoadmapOrder::CLI_VALUES
        );
    }
    Ok(())
}

pub fn render_drill(value: &serde_json::Value) -> String {
    let number = value["outcome_id"].as_u64().unwrap_or(0);
    let name = value["outcome_name"]
        .as_str()
        .unwrap_or("(unknown outcome)");
    let mut out = format!("#{number} {name}\n");
    let drill = &value["drill"];
    if let Some(convene) = drill["convene"].as_object() {
        let next = convene["next_iteration_name"].as_str().unwrap_or("");
        let gaps = convene["next_gap_count"].as_u64().unwrap_or(0);
        let then = match convene["then_iteration_name"].as_str() {
            Some(t) => format!(" · then {t}"),
            None => String::new(),
        };
        out.push_str(&format!("  convene? next: {next} — {gaps} gaps{then}\n"));
    }
    out.push_str("  iterations (priority order):\n");
    let iterations = drill["iterations"].as_array().cloned().unwrap_or_default();
    if iterations.is_empty() {
        out.push_str("    (none yet)\n");
        return out;
    }
    for iteration in &iterations {
        out.push_str(&render_drill_iteration(iteration));
    }
    out
}

fn render_drill_iteration(iteration: &serde_json::Value) -> String {
    let order = iteration["order"].as_u64().unwrap_or(0);
    let number = iteration["id"].as_u64().unwrap_or(0);
    let name = iteration["name"].as_str().unwrap_or("");
    let state = iteration["state"].as_str().unwrap_or("");
    let gap_count = iteration["gap_count"].as_u64().unwrap_or(0);
    let gap_part = if gap_count > 0 && state != "not-started" && state != "done" {
        format!("   {gap_count} gaps")
    } else {
        String::new()
    };
    let mut line = format!("    {order}. #{number} {name}   [{state}]{gap_part}\n");
    let checks = iteration["checks"].as_array().cloned().unwrap_or_default();
    if !checks.is_empty() {
        let rendered: Vec<String> = checks.iter().map(render_check).collect();
        line.push_str(&format!("         {}\n", rendered.join(" · ")));
    }
    line
}

fn render_check(check: &serde_json::Value) -> String {
    let label = check["label"].as_str().unwrap_or("");
    let state = match check["state"].as_str().unwrap_or("") {
        "met" => "met",
        "gap" => "GAP",
        "pending" => "pending",
        other => other,
    };
    format!("{label} {state}")
}
