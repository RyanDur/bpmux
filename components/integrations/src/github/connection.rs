use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubConnection {
    Hosted {
        owner: String,
        repo: String,
        token: String,
    },
    Enterprise {
        owner: String,
        repo: String,
        base_url: String,
        token: String,
    },
    WithBaseUrl {
        owner: String,
        repo: String,
        base_url: String,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("secret store: {0}")]
pub struct SecretError(pub String);

pub trait SecretStore: Send + Sync {
    fn store(&self, key: &str, value: &str) -> Result<(), SecretError>;
    fn load(&self, key: &str) -> Result<Option<String>, SecretError>;
    fn delete(&self, key: &str) -> Result<(), SecretError>;
}

pub const GITHUB_TOKEN_KEY: &str = "github-token";

impl GithubConnection {
    pub fn owner_repo(&self) -> (&str, &str) {
        match self {
            GithubConnection::Hosted { owner, repo, .. }
            | GithubConnection::Enterprise { owner, repo, .. }
            | GithubConnection::WithBaseUrl { owner, repo, .. } => (owner, repo),
        }
    }
}
