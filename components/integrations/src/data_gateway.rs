use std::sync::Arc;

use thiserror::Error;

use crate::db;
use crate::github::connection::{GITHUB_TOKEN_KEY, GithubConnection, SecretError, SecretStore};
use database_support::DatabaseGateway;

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("github-connection database: {0}")]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Secret(#[from] SecretError),
}

pub struct GithubConnectionDataGateway {
    gateway: Arc<DatabaseGateway>,
    secrets: Arc<dyn SecretStore>,
}

impl GithubConnectionDataGateway {
    pub fn new(gateway: Arc<DatabaseGateway>, secrets: Arc<dyn SecretStore>) -> Self {
        Self { gateway, secrets }
    }

    pub fn save(&self, connection: &GithubConnection) -> Result<(), ConnectionError> {
        let row = match connection {
            GithubConnection::Hosted { owner, repo, token } => {
                self.secrets.store(GITHUB_TOKEN_KEY, token)?;
                db::ConnectionRow {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    base_url: None,
                }
            }
            GithubConnection::WithBaseUrl {
                owner,
                repo,
                base_url,
            } => db::ConnectionRow {
                owner: owner.clone(),
                repo: repo.clone(),
                base_url: Some(base_url.clone()),
            },
        };
        db::save_connection(&self.gateway.lock(), &row)?;
        Ok(())
    }

    pub fn resolve(&self) -> Result<Option<GithubConnection>, ConnectionError> {
        let Some(row) = db::load_connection(&self.gateway.lock())? else {
            return Ok(None);
        };
        match row.base_url {
            Some(base_url) => Ok(Some(GithubConnection::WithBaseUrl {
                owner: row.owner,
                repo: row.repo,
                base_url,
            })),
            None => match self.secrets.load(GITHUB_TOKEN_KEY)? {
                Some(token) => Ok(Some(GithubConnection::Hosted {
                    owner: row.owner,
                    repo: row.repo,
                    token,
                })),
                None => {
                    diag::log_diag(
                        "github-connection: row present but no token in the secret store — treating as unconnected",
                    );
                    Ok(None)
                }
            },
        }
    }

    pub fn disconnect(&self) -> Result<(), ConnectionError> {
        db::delete_connection(&self.gateway.lock())?;
        self.secrets.delete(GITHUB_TOKEN_KEY)?;
        Ok(())
    }
}
