/// Re-exported for existing callers; the read moved to `crate::roadmap`
/// so `bridge` and `terminal` share a single source.
pub(in crate::terminal) use crate::roadmap::fetch_roadmap;

pub(in crate::terminal) fn fetch_board() -> Result<String, String> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args([
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "50",
        "--json",
        "number,title,state,labels",
    ]);
    if let Ok(repo) = std::env::var("BPMUX_REPO") {
        cmd.args(["-R", &repo]);
    } else {
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    }
    let output = cmd.output().map_err(|e| format!("failed to run gh: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh failed: {stderr}"));
    }
    let json: Vec<serde_json::Value> =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("parse error: {e}"))?;

    struct BoardItem {
        number: u64,
        title: String,
        status: Option<String>,
    }

    let mut active: Vec<BoardItem> = Vec::new();
    let mut backlog: Vec<BoardItem> = Vec::new();
    let mut icebox: Vec<BoardItem> = Vec::new();

    for issue in &json {
        let num = issue["number"].as_u64().unwrap_or(0);
        let title = issue["title"].as_str().unwrap_or("?").to_string();
        let state = issue["state"].as_str().unwrap_or("OPEN");
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();

        if state == "CLOSED" {
            continue;
        }

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
            number: num,
            title,
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
