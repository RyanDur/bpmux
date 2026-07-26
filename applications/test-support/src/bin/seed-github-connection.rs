use std::sync::Arc;

use database_support::DatabaseGateway;
use integrations::data_gateway::GithubConnectionDataGateway;
use integrations::github::connection::GithubConnection;
use integrations::in_memory::InMemorySecretStore;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [base, repo_spec, base_url] = args.as_slice() else {
        eprintln!("usage: seed-github-connection <data_base_dir> <owner/repo> <base_url>");
        return std::process::ExitCode::FAILURE;
    };
    let Some((owner, repo)) = repo_spec.split_once('/') else {
        eprintln!("seed-github-connection: repo must be owner/repo, got {repo_spec}");
        return std::process::ExitCode::FAILURE;
    };
    match seed(base, owner, repo, base_url) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("seed-github-connection: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn seed(base: &str, owner: &str, repo: &str, base_url: &str) -> Result<(), String> {
    std::fs::create_dir_all(base).map_err(|e| format!("create {base}: {e}"))?;
    let data_dir = location::durable_data_dir(std::path::Path::new(base))
        .map_err(|e| format!("durable data dir: {e}"))?;
    let db = Arc::new(DatabaseGateway::open(&data_dir).map_err(|e| format!("open db: {e}"))?);
    GithubConnectionDataGateway::new(db, Arc::new(InMemorySecretStore::default()))
        .save(&GithubConnection::WithBaseUrl {
            owner: owner.to_string(),
            repo: repo.to_string(),
            base_url: base_url.to_string(),
        })
        .map_err(|e| format!("save connection: {e}"))
}
