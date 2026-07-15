use super::board_render::{BoardScope, build_board_text};

pub(crate) async fn handle_view_board(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    let r = refresh.ok_or(
        "view_board requires refresh (RefreshDriver) — tracker fallback retired in slice 5l",
    )?;
    let mut stories = select.select(|s| product_management::story::slice::all(&s.stories));
    if select.select(|s| product_management::story::slice::is_empty(&s.stories)) {
        match r.refresh_all().await {
            Ok(()) => {
                stories = select.select(|s| product_management::story::slice::all(&s.stories));
                if stories.is_empty() {
                    diag::log_diag(
                        "view_board: empty store; refresh_all returned Ok with 0 stories — ingest succeeded but source-of-truth empty OR decode dropped all rows",
                    );
                } else {
                    diag::log_diag(&format!(
                        "view_board: empty store; refresh_all recovered {} stories",
                        stories.len()
                    ));
                }
            }
            Err(e) => {
                diag::log_diag(&format!("view_board: empty store; refresh_all failed: {e}"));
                stories = select.select(|s| product_management::story::slice::all(&s.stories));
            }
        }
    }
    let middle = workflow::board_middle_stage_ids(workflow::current_workflow());
    let scope = BoardScope {
        state: match args.get("state").and_then(|v| v.as_str()) {
            Some(s) => Some(parse_state(s, &middle).ok_or_else(|| {
                let middle_tokens = middle
                    .iter()
                    .map(|stage| stage.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "view_board: unknown state '{s}' (valid: icebox, ready, {middle_tokens}, delivered, accepted, retired)"
                )
            })?),
            None => None,
        },
        iteration: args.get("iteration").and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.trim_start_matches('#').parse().ok()))
        }),
        row_limit: args
            .get("row_limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
    };
    let (epics, iterations, fine) = select.select(|s| {
        let fine: std::collections::HashMap<product_management::story::StoryId, workflow::StageId> =
            stories
                .iter()
                .filter_map(|story| {
                    board::fine_stage(story, &s.board).map(|stage| (story.id, stage.id()))
                })
                .collect();
        (
            product_management::epic::all(&s.epics),
            product_management::iteration::all(&s.iterations),
            fine,
        )
    });
    Ok(build_board_text(
        &epics,
        &stories,
        &iterations,
        &scope,
        &fine,
        &middle,
    ))
}

fn parse_state(s: &str, middle: &[workflow::StageId]) -> Option<board::Column> {
    use board::Column;
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "icebox" => return Some(Column::Icebox),
        "ready" => return Some(Column::Ready),
        "acceptance" | "delivered" => return Some(Column::Acceptance),
        "accepted" => return Some(Column::Accepted),
        "retired" => return Some(Column::Retired),
        _ => {}
    }
    let normalized = lower.replace([' ', '_'], "-");
    middle
        .iter()
        .find(|stage| stage.as_str() == normalized)
        .map(|stage| Column::Middle(stage.clone()))
}
