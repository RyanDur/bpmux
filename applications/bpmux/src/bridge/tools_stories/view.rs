use super::board_render::{BoardScope, build_board_text};

pub(crate) async fn handle_view_board(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    let r = refresh.ok_or(
        "view_board requires refresh (RefreshDriver) — tracker fallback retired in slice 5l",
    )?;
    let mut stories = r.all_stories();
    if r.store_is_empty() {
        match r.refresh_all().await {
            Ok(()) => {
                stories = r.all_stories();
                if stories.is_empty() {
                    crate::diag::log_diag(
                        "view_board: empty store; refresh_all returned Ok with 0 stories — ingest succeeded but source-of-truth empty OR decode dropped all rows",
                    );
                } else {
                    crate::diag::log_diag(&format!(
                        "view_board: empty store; refresh_all recovered {} stories",
                        stories.len()
                    ));
                }
            }
            Err(e) => {
                crate::diag::log_diag(&format!("view_board: empty store; refresh_all failed: {e}"));
                stories = r.all_stories();
            }
        }
    }
    let middle = crate::workflow::board_middle_stage_ids(crate::workflow::current_workflow());
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
    let epics = r.epics();
    let iterations = r.iterations();
    Ok(build_board_text(
        &epics,
        &stories,
        &iterations,
        &scope,
        &middle,
    ))
}

fn parse_state(
    s: &str,
    middle: &[crate::workflow::StageId],
) -> Option<crate::product_management::story::BoardState> {
    use crate::product_management::story::BoardState;
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "icebox" => return Some(BoardState::Icebox),
        "ready" => return Some(BoardState::Ready),
        "acceptance" | "delivered" => return Some(BoardState::Acceptance),
        "accepted" => return Some(BoardState::Accepted),
        "retired" => return Some(BoardState::Retired),
        _ => {}
    }
    let normalized = lower.replace([' ', '_'], "-");
    middle
        .iter()
        .find(|stage| stage.as_str() == normalized)
        .map(|stage| BoardState::InProgress(stage.clone()))
}
