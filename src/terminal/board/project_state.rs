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

/// Fetch label-derived project state (how many open issues are
/// started/finished/ready/delivered/backlog). Returns `None` on any
/// auth / network / env error — the caller treats absence as "no
/// update, keep last state," matching the pre-#320 behavior under
/// gh failures.
///
/// #320: ported from `gh issue list` to octocrab. Async so it can
/// share the event loop thread with the other board fetchers; the
/// caller in `event.rs` now `tokio::spawn`s instead of
/// `spawn_blocking`.
pub async fn fetch_project_state() -> Option<ProjectState> {
    let (client, owner, repo) = crate::github::octocrab_client().ok()?;
    let page = client
        .issues(&owner, &repo)
        .list()
        .state(octocrab::params::State::Open)
        .per_page(100)
        .send()
        .await
        .ok()?;
    let mut state = ProjectState::default();
    for issue in &page.items {
        let names: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();
        if names.contains(&"delivered") {
            state.delivered += 1;
        } else if names.contains(&"started") || names.contains(&"finished") {
            state.active += 1;
        } else if names.contains(&"ready") {
            state.ready += 1;
        } else if names.contains(&"backlog") {
            state.backlog += 1;
        }
    }
    Some(state)
}
