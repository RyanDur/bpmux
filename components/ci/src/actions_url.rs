pub use message::GITHUB_BASE_URL;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionsUrl(String);

impl ActionsUrl {
    pub fn for_repo(repo: Option<(&str, &str)>) -> Self {
        match repo {
            Some((owner, name)) => Self(format!("{GITHUB_BASE_URL}/{owner}/{name}/actions")),
            None => Self(GITHUB_BASE_URL.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
