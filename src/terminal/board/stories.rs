use super::model::{BoardStory, StoryState};

/// Fetch all open + recently closed issues and categorize them into
/// board columns. Returns stories sorted by state priority.
///
/// When `BPMUX_BOARD_FIXTURE` env var is set, reads from that file
/// instead of calling gh. This lets e2e tests exercise the full
/// board rendering pipeline with controlled data.
pub fn fetch_board_stories() -> Result<Vec<BoardStory>, String> {
    // Test fixture support: read from file instead of gh
    if let Ok(fixture_path) = std::env::var("BPMUX_BOARD_FIXTURE") {
        let contents =
            std::fs::read_to_string(&fixture_path).map_err(|e| format!("fixture read: {e}"))?;
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&contents).map_err(|e| format!("fixture parse: {e}"))?;
        return parse_gh_issues(&json);
    }
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
    parse_gh_issues(&json)
}

/// Parse GitHub issue JSON into BoardStory structs.
fn parse_gh_issues(json: &[serde_json::Value]) -> Result<Vec<BoardStory>, String> {
    let mut stories = Vec::new();
    for issue in json {
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
