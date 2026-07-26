use std::sync::{Arc, RwLock};

pub use message::GITHUB_BASE_URL;

#[derive(Debug, Clone)]
pub struct ActionsUrl {
    current: Arc<RwLock<String>>,
}

impl ActionsUrl {
    pub fn for_repo(repo: Option<(&str, &str)>) -> Self {
        Self {
            current: Arc::new(RwLock::new(resolve(repo))),
        }
    }

    pub fn set_repo(&self, repo: Option<(&str, &str)>) {
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = resolve(repo);
    }

    pub fn current(&self) -> String {
        self.current
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

fn resolve(repo: Option<(&str, &str)>) -> String {
    match repo {
        Some((owner, name)) => format!("{GITHUB_BASE_URL}/{owner}/{name}/actions"),
        None => GITHUB_BASE_URL.to_string(),
    }
}
