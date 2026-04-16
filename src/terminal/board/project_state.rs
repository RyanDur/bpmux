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
