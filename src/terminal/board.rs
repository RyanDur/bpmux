/// Story state as shown in the board columns (#193).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryState {
    Ready,
    InProgress,
    Delivered,
    Accepted,
    Deferred,
    Icebox,
}

impl StoryState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::InProgress => "IN PROGRESS",
            Self::Delivered => "DELIVERED",
            Self::Accepted => "ACCEPTED",
            Self::Deferred => "DEFERRED",
            Self::Icebox => "ICEBOX",
        }
    }

    /// Key used in the priority.json file.
    pub fn priority_key(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::InProgress => "in_progress",
            Self::Delivered => "delivered",
            Self::Accepted => "accepted",
            Self::Deferred => "deferred",
            Self::Icebox => "icebox",
        }
    }
}

/// A story on the board, parsed from a GitHub issue.
#[derive(Debug, Clone)]
pub struct BoardStory {
    pub number: u64,
    pub title: String,
    pub state: StoryState,
    pub body: String,
    pub assignees: Vec<String>,
    pub created: String,
    /// For deferred stories, the reason (extracted from label description or body).
    pub reason: String,
    pub labels: Vec<String>,
}

impl BoardStory {
    /// Short display name: `#NNN title` truncated to fit.
    pub fn short_name(&self, max_len: usize) -> String {
        let prefix = format!("#{} ", self.number);
        let budget = max_len.saturating_sub(prefix.len());
        if self.title.len() > budget {
            format!(
                "{prefix}{}…",
                self.title
                    .chars()
                    .take(budget.saturating_sub(1))
                    .collect::<String>()
            )
        } else {
            format!("{prefix}{}", self.title)
        }
    }
}

/// Fetch all open + recently closed issues and categorize them into
/// board columns. Returns stories sorted by state priority.
pub fn fetch_board_stories() -> Result<Vec<BoardStory>, String> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args([
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "200",
        "--json",
        "number,title,state,labels,body,assignees,createdAt",
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

    let mut stories = Vec::new();
    for issue in &json {
        let num = issue["number"].as_u64().unwrap_or(0);
        let title = issue["title"].as_str().unwrap_or("?").to_string();
        let gh_state = issue["state"].as_str().unwrap_or("OPEN");
        let body = issue["body"].as_str().unwrap_or("").to_string();
        let assignees: Vec<String> = issue["assignees"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a["login"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let created = issue["createdAt"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(10) // YYYY-MM-DD
            .collect::<String>();
        let labels: Vec<String> = issue["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let has = |name: &str| labels.iter().any(|l| l == name);

        let state = if gh_state == "CLOSED" {
            StoryState::Accepted
        } else if has("deferred") {
            StoryState::Deferred
        } else if has("delivered") {
            StoryState::Delivered
        } else if has("started") || has("finished") {
            StoryState::InProgress
        } else if has("ready") {
            StoryState::Ready
        } else if has("icebox") {
            StoryState::Icebox
        } else {
            // Unlabeled open issues go to icebox
            StoryState::Icebox
        };

        // Extract deferred reason from labels or body first line
        let reason = if state == StoryState::Deferred {
            labels
                .iter()
                .find(|l| l.starts_with("deferred:"))
                .map(|l| l.strip_prefix("deferred:").unwrap_or(l).trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        stories.push(BoardStory {
            number: num,
            title,
            state,
            body,
            assignees,
            created,
            reason,
            labels,
        });
    }

    // Sort: Ready, InProgress, Delivered first (active work), then Accepted, Deferred, Icebox
    stories.sort_by_key(|s| match s.state {
        StoryState::Ready => 0,
        StoryState::InProgress => 1,
        StoryState::Delivered => 2,
        StoryState::Deferred => 3,
        StoryState::Accepted => 4,
        StoryState::Icebox => 5,
    });

    Ok(stories)
}

/// Group stories by state for the column layout, sorted by
/// PM-set priority order from `~/.bpmux/priority.json`.
pub fn stories_by_state(stories: &[BoardStory]) -> Vec<(StoryState, Vec<&BoardStory>)> {
    use crate::session::priority;
    use StoryState::*;

    let priority_index = priority::default_priority_path()
        .map(|p| priority::load_priority(&p))
        .unwrap_or_default();

    let states = [Ready, InProgress, Delivered, Accepted, Deferred, Icebox];
    states
        .iter()
        .filter_map(|&st| {
            let group: Vec<&BoardStory> = stories.iter().filter(|s| s.state == st).collect();
            if group.is_empty() {
                return None;
            }
            // Apply priority ordering
            let numbers: Vec<u64> = group.iter().map(|s| s.number).collect();
            let prio = priority_index
                .columns
                .get(st.priority_key())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let ordered = priority::sort_by_priority(&numbers, prio);
            let sorted: Vec<&BoardStory> = ordered
                .iter()
                .filter_map(|&n| group.iter().find(|s| s.number == n).copied())
                .collect();
            Some((st, sorted))
        })
        .collect()
}

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
    } else {
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
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
    } else {
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
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
