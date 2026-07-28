use std::sync::Arc;

use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;

use integrations::data_gateway::{ConnectionError, GithubConnectionDataGateway};
use integrations::github::connection::GithubConnection;

use super::shared::{JsonResponse, TrackerCell, err_body, json_headers, owner_walk_or_forbidden};

pub type ConnectionsHandle = Arc<GithubConnectionDataGateway>;

fn ok_body(body: serde_json::Value) -> JsonResponse {
    (StatusCode::OK, json_headers(), body.to_string())
}

fn bad_request(message: &str) -> JsonResponse {
    (StatusCode::BAD_REQUEST, json_headers(), err_body(message))
}

fn storage_failure(context: &str, e: &ConnectionError) -> JsonResponse {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        json_headers(),
        err_body(&format!("{context}: {e}")),
    )
}

fn connection_body(connection: &GithubConnection) -> serde_json::Value {
    match connection {
        GithubConnection::Hosted { owner, repo, .. } => {
            serde_json::json!({"state": "connected", "owner": owner, "repo": repo})
        }
        GithubConnection::Enterprise {
            owner,
            repo,
            base_url,
            ..
        }
        | GithubConnection::WithBaseUrl {
            owner,
            repo,
            base_url,
        } => serde_json::json!({
            "state": "connected", "owner": owner, "repo": repo, "base_url": base_url
        }),
    }
}

pub async fn get_github_connection(
    axum::Extension(connections): axum::Extension<ConnectionsHandle>,
) -> impl IntoResponse {
    match connections.resolve() {
        Ok(None) => ok_body(serde_json::json!({"state": "disconnected"})),
        Ok(Some(connection)) => ok_body(connection_body(&connection)),
        Err(e) => storage_failure("github-connection status failed", &e),
    }
}

#[derive(Deserialize)]
pub struct ConnectPayload {
    pub owner: String,
    pub repo: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

fn candidate_from(payload: &ConnectPayload) -> Result<GithubConnection, JsonResponse> {
    let owner = payload.owner.trim().to_string();
    let repo = payload.repo.trim().to_string();
    if owner.is_empty() || repo.is_empty() {
        return Err(bad_request("owner and repo must not be empty"));
    }
    let base_url = payload
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty());
    let token = payload
        .token
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());
    match (base_url, token) {
        (Some(base_url), Some(token)) => Ok(GithubConnection::Enterprise {
            owner,
            repo,
            base_url: base_url.to_string(),
            token: token.to_string(),
        }),
        (Some(base_url), None) => Ok(GithubConnection::WithBaseUrl {
            owner,
            repo,
            base_url: base_url.to_string(),
        }),
        (None, Some(token)) => Ok(GithubConnection::Hosted {
            owner,
            repo,
            token: token.to_string(),
        }),
        (None, None) => Err(bad_request("a hosted connection needs a token")),
    }
}

pub async fn put_github_connection(
    axum::Extension(connections): axum::Extension<ConnectionsHandle>,
    axum::Extension(select): axum::Extension<store::Select>,
    axum::Extension(actions_url): axum::Extension<ci::ActionsUrl>,
    axum::Extension(cell): axum::Extension<TrackerCell>,
    axum::Extension(orchestrator): axum::Extension<sync::OrchestratorHandle>,
    headers: HeaderMap,
    Json(payload): Json<ConnectPayload>,
) -> impl IntoResponse {
    if let Err(refusal) = owner_walk_or_forbidden(&select, &headers, "connecting github") {
        return refusal;
    }
    let candidate = match candidate_from(&payload) {
        Ok(candidate) => candidate,
        Err(refusal) => return refusal,
    };
    if let Err(e) = connections
        .disconnect()
        .and_then(|()| connections.save(&candidate))
    {
        return storage_failure("persisting the connection failed", &e);
    }
    let (owner, repo) = candidate.owner_repo();
    actions_url.set_repo(Some((owner, repo)));
    match integrations::github::GitHubSourceAdapter::from_connection(&candidate, cell.observer()) {
        Some(adapter) => {
            cell.connect(Arc::new(adapter));
            if let Some(orchestrator) = orchestrator {
                tokio::spawn(async move {
                    if let Err(e) = orchestrator.refresh_all().await {
                        diag::log_diag(&format!(
                            "github-connection: first sync after connect failed ({e}) — the poll loop retries"
                        ));
                    }
                });
            }
        }
        None => {
            cell.disconnect();
            diag::log_diag(
                "github-connection: stored settings do not build a client — running unconnected until they change",
            );
        }
    }
    ok_body(connection_body(&candidate))
}

pub async fn delete_github_connection(
    axum::Extension(connections): axum::Extension<ConnectionsHandle>,
    axum::Extension(select): axum::Extension<store::Select>,
    axum::Extension(actions_url): axum::Extension<ci::ActionsUrl>,
    axum::Extension(cell): axum::Extension<TrackerCell>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(refusal) = owner_walk_or_forbidden(&select, &headers, "disconnecting github") {
        return refusal;
    }
    if let Err(e) = connections.disconnect() {
        return storage_failure("disconnect failed", &e);
    }
    actions_url.set_repo(None);
    cell.disconnect();
    ok_body(serde_json::json!({"state": "disconnected"}))
}
