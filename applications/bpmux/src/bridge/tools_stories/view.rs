pub(crate) async fn handle_view_board(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
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
    let scope = BoardScope {
        state: match args.get("state").and_then(|v| v.as_str()) {
            Some(s) => Some(parse_bucket(s).ok_or_else(|| {
                format!(
                    "view_board: unknown state '{s}' (valid: icebox, ready, in_progress, desk_check, delivered, accepted, retired)"
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
    Ok(build_board_text(&epics, &stories, &scope))
}

fn parse_bucket(s: &str) -> Option<crate::product_management::board::column::ColumnBucket> {
    use crate::product_management::board::column::ColumnBucket;
    match s.to_ascii_lowercase().as_str() {
        "icebox" => Some(ColumnBucket::Icebox),
        "ready" => Some(ColumnBucket::Ready),
        "in_progress" | "in-progress" | "inprogress" => Some(ColumnBucket::InProgress),
        "desk_check" | "desk-check" | "deskcheck" => Some(ColumnBucket::DeskCheck),
        "delivered" => Some(ColumnBucket::Delivered),
        "accepted" => Some(ColumnBucket::Accepted),
        "retired" => Some(ColumnBucket::Retired),
        _ => None,
    }
}

#[derive(Default)]
pub(in crate::bridge) struct BoardScope {
    pub state: Option<crate::product_management::board::column::ColumnBucket>,
    pub iteration: Option<u64>,
    pub row_limit: Option<usize>,
}

pub(in crate::bridge) fn build_board_text(
    epics: &[crate::product_management::epic::Epic],
    stories: &[crate::product_management::story::Story],
    scope: &BoardScope,
) -> String {
    let format_entry = |s: &crate::product_management::story::Story| -> (u64, String) {
        let parent_epic = s.parent_epic();
        let prefix = if parent_epic.is_some() { "↳ " } else { "" };
        let suffix = match parent_epic {
            Some(n) => format!(" (epic #{})", n.value()),
            None => String::new(),
        };
        let acceptor = match s.acceptor() {
            Some(a) => format!(" · awaiting {}", a.as_str()),
            None => String::new(),
        };
        (
            s.id.value(),
            format!(
                "{prefix}#{} {}{}{}",
                s.id.value(),
                s.title,
                suffix,
                acceptor
            ),
        )
    };
    use crate::product_management::board::column::{ColumnBucket, column_for};
    let in_scope_iteration = |s: &crate::product_management::story::Story| -> bool {
        match scope.iteration {
            Some(n) => {
                s.iteration_membership().map(|(it, _)| it)
                    == Some(crate::product_management::iteration::IterationId::new(n))
            }
            None => true,
        }
    };
    let bucket_section = |bucket: ColumnBucket| -> (usize, Vec<String>) {
        let mut entries: Vec<(u64, String)> = stories
            .iter()
            .filter(|s| in_scope_iteration(s))
            .filter(|s| column_for(s.state, s.is_closed) == bucket)
            .map(format_entry)
            .collect();
        entries.sort_by_key(|(n, _)| *n);
        let full_count = entries.len();
        let mut items: Vec<String> = entries.into_iter().map(|(_, e)| e).collect();
        if let Some(limit) = scope.row_limit {
            items.truncate(limit);
        }
        (full_count, items)
    };
    let mut out = String::new();
    if scope.state.is_none() && scope.iteration.is_none() {
        let mut epics: Vec<&crate::product_management::epic::Epic> = epics.iter().collect();
        epics.sort_by_key(|e| e.id);
        if !epics.is_empty() {
            out.push_str(&format!("EPICS ({})\n", epics.len()));
            for epic in &epics {
                let children: Vec<&crate::product_management::story::Story> = stories
                    .iter()
                    .filter(|c| c.parent_epic() == Some(epic.id))
                    .collect();
                let counts = crate::product_management::epic::derive_epic_counts(&children);
                let derived =
                    crate::product_management::epic::derive_epic_state(epic.state, &children);
                out.push_str(&format!(
                    "  #{} [{}] {} ({total} children: {accepted} accepted, {retired} retired, {active} active, {icebox} icebox)\n",
                    epic.id.value(),
                    derived.as_str(),
                    epic.title,
                    total = counts.total_count,
                    accepted = counts.accepted_count,
                    retired = counts.retired_count,
                    active = counts.active_count,
                    icebox = counts.icebox_count
                ));
            }
            out.push('\n');
        }
    }
    let section = |name: &str, full_count: usize, items: &[String], out: &mut String| {
        out.push_str(&format!("{name} ({full_count})\n"));
        for item in items {
            out.push_str(&format!("  {item}\n"));
        }
        out.push('\n');
    };
    for bucket in [
        ColumnBucket::Icebox,
        ColumnBucket::Ready,
        ColumnBucket::InProgress,
        ColumnBucket::DeskCheck,
        ColumnBucket::Delivered,
        ColumnBucket::Accepted,
        ColumnBucket::Retired,
    ] {
        if scope.state.is_some_and(|only| only != bucket) {
            continue;
        }
        let (full_count, items) = bucket_section(bucket);
        section(bucket.header(), full_count, &items, &mut out);
    }
    out
}
