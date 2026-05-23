pub(crate) async fn handle_view_board(
    _args: &serde_json::Map<String, serde_json::Value>,
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
    let bucket_for = |bucket: ColumnBucket| -> Vec<String> {
        let mut entries: Vec<(u64, String)> = stories
            .iter()
            .filter(|s| s.story_type != crate::board::domain::StoryType::Epic)
            .filter(|s| column_for(s.state, s.is_closed) == bucket)
            .map(format_entry)
            .collect();
        entries.sort_by_key(|(n, _)| *n);
        entries.into_iter().map(|(_, e)| e).collect()
    };
    let icebox = bucket_for(ColumnBucket::Icebox);
    let ready = bucket_for(ColumnBucket::Ready);
    let in_progress = bucket_for(ColumnBucket::InProgress);
    let desk_check = bucket_for(ColumnBucket::DeskCheck);
    let delivered = bucket_for(ColumnBucket::Delivered);
    let accepted = bucket_for(ColumnBucket::Accepted);
    let retired = bucket_for(ColumnBucket::Retired);
    let mut epics: Vec<&crate::board::domain::Story> = stories
        .iter()
        .filter(|s| s.story_type == crate::board::domain::StoryType::Epic)
        .collect();
    epics.sort_by_key(|s| s.number);
    let mut out = String::new();
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
    let section = |name: &str, items: &[String], out: &mut String| {
        out.push_str(&format!("{name} ({})\n", items.len()));
        for item in items {
            out.push_str(&format!("  {item}\n"));
        }
        out.push('\n');
    };
    section(ColumnBucket::Icebox.header(), &icebox, &mut out);
    section(ColumnBucket::Ready.header(), &ready, &mut out);
    section(ColumnBucket::InProgress.header(), &in_progress, &mut out);
    section(ColumnBucket::DeskCheck.header(), &desk_check, &mut out);
    section(ColumnBucket::Delivered.header(), &delivered, &mut out);
    section(ColumnBucket::Accepted.header(), &accepted, &mut out);
    section(ColumnBucket::Retired.header(), &retired, &mut out);
    Ok(out)
}
