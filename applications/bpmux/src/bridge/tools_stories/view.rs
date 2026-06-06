pub(crate) async fn handle_view_board(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let r = refresh.ok_or(
        "view_board requires refresh (RefreshDriver) — tracker fallback retired in slice 5l",
    )?;
    let mut stories = r.all_stories();
    if stories.is_empty() {
        match r.refresh_all().await {
            Ok(()) => {
                stories = r.all_stories();
                if stories.is_empty() {
                    crate::session::diag::log_diag(
                        "view_board: empty store; refresh_all returned Ok with 0 stories — ingest succeeded but source-of-truth empty OR decode dropped all rows",
                    );
                } else {
                    crate::session::diag::log_diag(&format!(
                        "view_board: empty store; refresh_all recovered {} stories",
                        stories.len()
                    ));
                }
            }
            Err(e) => {
                crate::session::diag::log_diag(&format!(
                    "view_board: empty store; refresh_all failed: {e}"
                ));
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
    Ok(build_board_text(&stories, &scope))
}

fn parse_bucket(s: &str) -> Option<crate::board::column::ColumnBucket> {
    use crate::board::column::ColumnBucket;
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
    pub state: Option<crate::board::column::ColumnBucket>,
    pub iteration: Option<u64>,
    pub row_limit: Option<usize>,
}

pub(in crate::bridge) fn build_board_text(
    stories: &[crate::board::domain::Story],
    scope: &BoardScope,
) -> String {
    let format_entry = |s: &crate::board::domain::Story| -> (u64, String) {
        let prefix = if s.parent_epic.is_some() { "↳ " } else { "" };
        let suffix = match s.parent_epic {
            Some(n) => format!(" (epic #{n})"),
            None => String::new(),
        };
        (
            s.number,
            format!("{prefix}#{} {}{}", s.number, s.title, suffix),
        )
    };
    use crate::board::column::{ColumnBucket, column_for};
    let in_scope_iteration = |s: &crate::board::domain::Story| -> bool {
        match scope.iteration {
            Some(n) => s.iteration_membership.map(|(it, _)| it) == Some(n),
            None => true,
        }
    };
    let bucket_section = |bucket: ColumnBucket| -> (usize, Vec<String>) {
        let mut entries: Vec<(u64, String)> = stories
            .iter()
            .filter(|s| s.story_type != crate::board::domain::StoryType::Epic)
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
        let mut epics: Vec<&crate::board::domain::Story> = stories
            .iter()
            .filter(|s| s.story_type == crate::board::domain::StoryType::Epic)
            .collect();
        epics.sort_by_key(|s| s.number);
        if !epics.is_empty() {
            out.push_str(&format!("EPICS ({})\n", epics.len()));
            for epic in &epics {
                let children: Vec<&crate::board::domain::Story> = stories
                    .iter()
                    .filter(|c| c.parent_epic == Some(epic.number))
                    .collect();
                let counts = crate::board::domain::derive_epic_counts(&children);
                let derived = crate::board::domain::derive_epic_state(epic.state, &children);
                out.push_str(&format!(
                    "  #{} [{}] {} ({total} children: {accepted} accepted, {retired} retired, {active} active, {icebox} icebox)\n",
                    epic.number,
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod scoped_tests {
    use super::*;
    use crate::board::Story;
    use crate::board::StoryState;
    use crate::board::column::ColumnBucket;
    use chrono::{TimeZone, Utc};

    fn story(num: u64, title: &str, state: StoryState) -> Story {
        Story::new_minimal(
            num,
            title.to_string(),
            state,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        )
    }

    fn in_iteration(mut s: Story, iteration: u64) -> Story {
        s.iteration_membership = Some((iteration, 1));
        s
    }

    #[test]
    fn default_scope_is_unchanged_full_board() {
        let stories = vec![
            story(1, "ready one", StoryState::Ready),
            story(2, "in-progress one", StoryState::InProgress),
        ];
        let out = build_board_text(&stories, &BoardScope::default());
        assert!(out.contains("#1 ready one"));
        assert!(out.contains("#2 in-progress one"));
        assert!(out.contains(ColumnBucket::Ready.header()));
        assert!(out.contains(ColumnBucket::InProgress.header()));
    }

    #[test]
    fn state_scope_renders_only_the_matching_bucket() {
        let stories = vec![
            story(1, "ready one", StoryState::Ready),
            story(2, "in-progress one", StoryState::InProgress),
        ];
        let scope = BoardScope {
            state: Some(ColumnBucket::Ready),
            ..BoardScope::default()
        };
        let out = build_board_text(&stories, &scope);
        assert!(out.contains("#1 ready one"));
        assert!(!out.contains("#2 in-progress one"));
        assert!(!out.contains(ColumnBucket::InProgress.header()));
    }

    #[test]
    fn iteration_scope_excludes_other_iterations() {
        let stories = vec![
            in_iteration(story(10, "iter-one story", StoryState::Ready), 1),
            in_iteration(story(20, "iter-two story", StoryState::Ready), 2),
        ];
        let scope = BoardScope {
            iteration: Some(1),
            ..BoardScope::default()
        };
        let out = build_board_text(&stories, &scope);
        assert!(out.contains("#10 iter-one story"));
        assert!(!out.contains("#20 iter-two story"));
    }

    #[test]
    fn row_limit_caps_entries_but_count_reflects_full_total() {
        let stories: Vec<Story> = (0..10)
            .map(|i| story(100 + i, &format!("ready {i}"), StoryState::Ready))
            .collect();
        let scope = BoardScope {
            state: Some(ColumnBucket::Ready),
            row_limit: Some(3),
            ..BoardScope::default()
        };
        let out = build_board_text(&stories, &scope);
        assert_eq!(out.matches("ready ").count(), 3);
        assert!(out.contains("(10)"));
    }

    #[test]
    fn large_board_scoped_is_ingestible_while_full_is_not() {
        let stories: Vec<Story> = (0..4000u64)
            .map(|i| {
                let title = format!(
                    "story {i} with a sufficiently long descriptive title to inflate the full board payload"
                );
                let state = if i % 2 == 0 {
                    StoryState::Ready
                } else {
                    StoryState::Accepted
                };
                story(1000 + i, &title, state)
            })
            .collect();
        let full = build_board_text(&stories, &BoardScope::default());
        let scoped = build_board_text(
            &stories,
            &BoardScope {
                state: Some(ColumnBucket::Ready),
                row_limit: Some(20),
                ..BoardScope::default()
            },
        );
        assert!(
            full.len() > 100_000,
            "full board should be large: {}",
            full.len()
        );
        assert!(
            scoped.len() < 10_000,
            "scoped board should be ingestible-whole: {}",
            scoped.len()
        );
        assert!(!scoped.contains(ColumnBucket::Accepted.header()));
    }
}
