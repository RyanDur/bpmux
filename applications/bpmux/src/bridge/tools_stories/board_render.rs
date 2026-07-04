#[derive(Default)]
pub(in crate::bridge) struct BoardScope {
    pub state: Option<crate::view::board::arrangement::ColumnBucket>,
    pub iteration: Option<u64>,
    pub row_limit: Option<usize>,
}

pub(in crate::bridge) fn build_board_text(
    epics: &[crate::product_management::epic::Epic],
    stories: &[crate::product_management::story::Story],
    iterations: &[crate::product_management::iteration::Iteration],
    scope: &BoardScope,
    middle: &[crate::workflow::StageId],
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
    use crate::view::board::arrangement::{ColumnBucket, column_for};
    let in_scope_iteration = |s: &crate::product_management::story::Story| -> bool {
        match scope.iteration {
            Some(n) => {
                let target = crate::product_management::iteration::IterationId::new(n);
                iterations
                    .iter()
                    .any(|it| it.id == target && it.stories.contains(&s.id))
            }
            None => true,
        }
    };
    let bucket_section = |bucket: ColumnBucket| -> (usize, Vec<String>) {
        let mut entries: Vec<(u64, String)> = stories
            .iter()
            .filter(|s| in_scope_iteration(s))
            .filter(|s| column_for(s.state.clone(), s.is_closed) == bucket)
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
                let derived = crate::product_management::epic::derive_epic_state(
                    epic.state.clone(),
                    &children,
                );
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
    let mut buckets = vec![ColumnBucket::Icebox, ColumnBucket::Ready];
    buckets.extend(middle.iter().cloned().map(ColumnBucket::Middle));
    buckets.extend([
        ColumnBucket::Acceptance,
        ColumnBucket::Accepted,
        ColumnBucket::Retired,
    ]);
    for bucket in buckets {
        if scope.state.as_ref().is_some_and(|only| *only != bucket) {
            continue;
        }
        let header = bucket.header();
        let (full_count, items) = bucket_section(bucket);
        section(&header, full_count, &items, &mut out);
    }
    out
}
