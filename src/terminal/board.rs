#[derive(Default, Clone)]
pub struct ProjectState {
    pub active: usize,
    pub ready: usize,
    pub delivered: usize,
    pub backlog: usize,
}

impl ProjectState {
    pub fn title_summary(&self) -> String {
        let mut parts = vec!["bpmux".to_string()];
        if self.active > 0 {
            parts.push(format!("{} wip", self.active));
        }
        if self.ready > 0 {
            parts.push(format!("{} ready", self.ready));
        }
        if self.delivered > 0 {
            parts.push(format!("{} delivered", self.delivered));
        }
        if parts.len() == 1 {
            return " bpmux ".to_string();
        }
        format!(" {} ", parts.join(" · "))
    }
}

pub fn fetch_project_state() -> Option<ProjectState> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args([
        "issue", "list", "--state", "open", "--limit", "100", "--json", "labels",
    ]);
    if let Ok(repo) = std::env::var("BPMUX_REPO") {
        cmd.args(["-R", &repo]);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let json: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).ok()?;

    let mut state = ProjectState::default();
    for issue in &json {
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();

        if labels.contains(&"delivered") {
            state.delivered += 1;
        } else if labels.contains(&"started") || labels.contains(&"finished") {
            state.active += 1;
        } else if labels.contains(&"ready") {
            state.ready += 1;
        } else if labels.contains(&"backlog") {
            state.backlog += 1;
        }
    }
    Some(state)
}

pub struct DoraMetrics {
    pub shipped_this_week: usize,
    pub in_progress: usize,
    pub rejection_rate: f64,
    pub avg_cycle_hours: f64,
}

impl DoraMetrics {
    pub fn format(&self) -> String {
        let mut lines = vec!["## Metrics (DORA)".to_string(), String::new()];
        lines.push(format!("  Shipped this week:  {}", self.shipped_this_week));
        lines.push(format!("  In progress:        {}", self.in_progress));
        lines.push(format!("  Rejection rate:     {:.1}%", self.rejection_rate));
        if self.avg_cycle_hours < 1.0 {
            lines.push(format!(
                "  Avg cycle time:     {:.0} min",
                self.avg_cycle_hours * 60.0
            ));
        } else {
            lines.push(format!(
                "  Avg cycle time:     {:.1} hrs",
                self.avg_cycle_hours
            ));
        }
        lines.join("\n")
    }
}

pub fn fetch_dora_metrics() -> Option<DoraMetrics> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args([
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "200",
        "--json",
        "state,labels,createdAt,closedAt",
    ]);
    if let Ok(repo) = std::env::var("BPMUX_REPO") {
        cmd.args(["-R", &repo]);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let issues: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).ok()?;

    let now = chrono::Utc::now();
    let week_ago = now - chrono::Duration::days(7);

    let mut shipped_this_week = 0usize;
    let mut in_progress = 0usize;
    let mut total_closed = 0usize;
    let mut rejected = 0usize;
    let mut cycle_times: Vec<f64> = Vec::new();

    for issue in &issues {
        let state = issue["state"].as_str().unwrap_or("OPEN");
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();

        if state == "CLOSED" {
            total_closed += 1;
            if labels.contains(&"rejected") {
                rejected += 1;
            }

            if let (Some(created), Some(closed)) = (
                issue["createdAt"]
                    .as_str()
                    .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok()),
                issue["closedAt"]
                    .as_str()
                    .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok()),
            ) {
                if closed > week_ago {
                    shipped_this_week += 1;
                }
                let hours = (closed - created).num_minutes() as f64 / 60.0;
                if hours >= 0.0 {
                    cycle_times.push(hours);
                }
            }
        } else if labels.contains(&"started") || labels.contains(&"finished") {
            in_progress += 1;
        }
    }

    let rejection_rate = if total_closed > 0 {
        rejected as f64 / total_closed as f64 * 100.0
    } else {
        0.0
    };

    let avg_cycle_hours = if cycle_times.is_empty() {
        0.0
    } else {
        cycle_times.iter().sum::<f64>() / cycle_times.len() as f64
    };

    Some(DoraMetrics {
        shipped_this_week,
        in_progress,
        rejection_rate,
        avg_cycle_hours,
    })
}

fn repo_root() -> Result<String, String> {
    if let Ok(root) = std::env::var("BPMUX_ROOT") {
        return Ok(root);
    }
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("failed to find repo root: {e}"))?;
    if !output.status.success() {
        return Err("not inside a git repository".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn fetch_roadmap() -> Result<String, String> {
    let root = repo_root()?;
    let path = std::path::Path::new(&root).join("ROADMAP.md");
    std::fs::read_to_string(&path).map_err(|e| format!("cannot read ROADMAP.md: {e}"))
}

pub(super) fn fetch_board() -> Result<String, String> {
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
        lines.push(format!("### ● Active ({})", active.len()));
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
        lines.push(format!("### ○ Backlog ({})", backlog.len()));
        for item in backlog.iter().take(10) {
            lines.push(format!("- **#{}** {}", item.number, item.title));
        }
        if backlog.len() > 10 {
            lines.push(format!("- *+{} more*", backlog.len() - 10));
        }
    }

    if !icebox.is_empty() {
        lines.push(String::new());
        lines.push(format!("### ◌ Icebox ({})", icebox.len()));
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
