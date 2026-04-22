/// Re-exported for existing callers; the read moved to `crate::roadmap`
/// so `bridge` and `terminal` share a single source.
pub(in crate::terminal) use crate::roadmap::fetch_roadmap;

/// Legacy project-panel fallback: renders a markdown summary of the
/// current board when the roadmap file can't be read.
///
/// #320: ported from `gh issue list` to octocrab. Async because it
/// talks to octocrab; callers in sync contexts bridge via
/// `Handle::current().block_on(...)` (see `event_dispatch::mouse`).
pub(in crate::terminal) async fn fetch_board() -> Result<String, String> {
    let (client, owner, repo) = crate::github::octocrab_client()?;
    let page = client
        .issues(&owner, &repo)
        .list()
        .state(octocrab::params::State::Open)
        .per_page(50)
        .send()
        .await
        .map_err(|e| crate::github::classify_error(&e).user_message())?;

    struct BoardItem {
        number: u64,
        title: String,
        status: Option<String>,
    }

    let mut active: Vec<BoardItem> = Vec::new();
    let mut backlog: Vec<BoardItem> = Vec::new();
    let mut icebox: Vec<BoardItem> = Vec::new();

    for issue in &page.items {
        // Only open issues were requested; no CLOSED filter needed.
        let labels: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();

        let status = if labels.contains(&"delivered") {
            Some("delivered".to_string())
        } else if labels.contains(&"finished") {
            Some("finished".to_string())
        } else if labels.contains(&"started") {
            Some("started".to_string())
        } else if labels.contains(&"rejected") {
            Some("rejected".to_string())
        } else {
            None
        };

        let item = BoardItem {
            number: issue.number,
            title: issue.title.clone(),
            status,
        };

        if labels.contains(&"icebox") {
            icebox.push(item);
        } else if labels.contains(&"backlog") {
            backlog.push(item);
        } else {
            active.push(item);
        }
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push("## Board".to_string());

    if !active.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "### {} Active ({})",
            crate::terminal::icons::ACTIVE,
            active.len()
        ));
        for item in &active {
            let status_tag = item
                .status
                .as_ref()
                .map(|s| format!(" *{s}*"))
                .unwrap_or_default();
            lines.push(format!("- **#{}** {}{status_tag}", item.number, item.title));
        }
    }

    if !backlog.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "### {} Backlog ({})",
            crate::terminal::icons::INACTIVE,
            backlog.len()
        ));
        for item in backlog.iter().take(10) {
            lines.push(format!("- **#{}** {}", item.number, item.title));
        }
        if backlog.len() > 10 {
            lines.push(format!("- *+{} more*", backlog.len() - 10));
        }
    }

    if !icebox.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "### {} Icebox ({})",
            crate::terminal::icons::PARKED,
            icebox.len()
        ));
        if let Some(first) = icebox.first() {
            lines.push(format!("- **#{}** {}", first.number, first.title));
        }
        if icebox.len() > 1 {
            lines.push(format!("- *+{} more*", icebox.len() - 1));
        }
    }

    if lines.len() <= 1 {
        Ok("No open issues found.".to_string())
    } else {
        Ok(lines.join("\n"))
    }
}
