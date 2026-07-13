use std::collections::HashMap;

use board::{Column, board_columns, group_by_column};
use product_management::story::StoryId;
use view::board::arrangement::column_header;
use workflow::StageId;

#[derive(Default)]
pub(in crate::bridge) struct BoardScope {
    pub state: Option<Column>,
    pub iteration: Option<u64>,
    pub row_limit: Option<usize>,
}

pub(in crate::bridge) fn build_board_text(
    epics: &[product_management::epic::Epic],
    stories: &[product_management::story::Story],
    iterations: &[product_management::iteration::Iteration],
    scope: &BoardScope,
    fine: &HashMap<StoryId, StageId>,
    middle: &[StageId],
) -> String {
    let format_entry = |s: &product_management::story::Story| -> (u64, String) {
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
    let in_scope_iteration = |s: &product_management::story::Story| -> bool {
        match scope.iteration {
            Some(n) => {
                let target = product_management::iteration::IterationId::new(n);
                iterations
                    .iter()
                    .any(|it| it.id == target && it.stories.contains(&s.id))
            }
            None => true,
        }
    };
    let board = group_by_column(
        stories
            .iter()
            .filter(|s| in_scope_iteration(s))
            .map(|s| (s, fine.get(&s.id).cloned())),
        iterations,
    );
    let column_section = |column: &Column| -> (usize, Vec<String>) {
        let mut entries: Vec<(u64, String)> = board
            .get(column)
            .into_iter()
            .flatten()
            .map(|&s| format_entry(s))
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
        let mut epics: Vec<&product_management::epic::Epic> = epics.iter().collect();
        epics.sort_by_key(|e| e.id);
        if !epics.is_empty() {
            out.push_str(&format!("EPICS ({})\n", epics.len()));
            for epic in &epics {
                let children = product_management::epic::children_of(stories, epic.id);
                let counts = product_management::epic::derive_epic_counts(&children, iterations);
                let derived = product_management::epic::derive_epic_state(&children, iterations);
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
    for column in board_columns(middle) {
        if scope.state.as_ref().is_some_and(|only| *only != column) {
            continue;
        }
        let (full_count, items) = column_section(&column);
        section(&column_header(&column), full_count, &items, &mut out);
    }
    out
}
