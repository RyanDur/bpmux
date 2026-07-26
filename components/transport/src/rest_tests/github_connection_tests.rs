#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tower::ServiceExt;

use crate::rest::shared::OWNER_NONCE_HEADER;
use crate::rest::{ConnectionsHandle, github_connection};
use database_support::DatabaseGateway;
use integrations::data_gateway::GithubConnectionDataGateway;
use integrations::github::connection::{GITHUB_TOKEN_KEY, GithubConnection, SecretStore};
use integrations::in_memory::InMemorySecretStore;
use owner_session::OwnerSessionAction;
use store_test_support::SeededStore;

struct Harness {
    router: axum::Router,
    connections: ConnectionsHandle,
    secrets: Arc<InMemorySecretStore>,
    seeded: SeededStore,
    actions_url: ci::ActionsUrl,
    _dir: TempDir,
}

fn harness() -> Harness {
    let dir = TempDir::new().expect("tempdir");
    let db = Arc::new(DatabaseGateway::open(dir.path()).expect("open temp database"));
    let secrets = Arc::new(InMemorySecretStore::default());
    let connections: ConnectionsHandle =
        Arc::new(GithubConnectionDataGateway::new(db, secrets.clone()));
    let seeded = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    let actions_url = ci::ActionsUrl::for_repo(None);
    let router = axum::Router::new()
        .route(
            "/api/github-connection",
            axum::routing::get(github_connection::get_github_connection)
                .put(github_connection::put_github_connection)
                .delete(github_connection::delete_github_connection),
        )
        .layer(axum::Extension(connections.clone()))
        .layer(axum::Extension(seeded.select()))
        .layer(axum::Extension(actions_url.clone()));
    Harness {
        router,
        connections,
        secrets,
        seeded,
        actions_url,
        _dir: dir,
    }
}

fn claim_owner_nonce(h: &Harness, nonce: &str) {
    h.seeded.dispatch(store::AppAction::OwnerSession(
        OwnerSessionAction::Claimed {
            nonce: nonce.to_string(),
        },
    ));
}

async fn send(
    router: axum::Router,
    method: &str,
    body: Option<serde_json::Value>,
    nonce: Option<&str>,
) -> axum::http::Response<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri("/api/github-connection")
        .header("content-type", "application/json");
    if let Some(nonce) = nonce {
        builder = builder.header(OWNER_NONCE_HEADER, nonce);
    }
    let request = builder
        .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
        .expect("build request");
    router.oneshot(request).await.expect("response")
}

async fn body_json(response: axum::http::Response<Body>) -> serde_json::Value {
    use http_body_util::BodyExt;
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json body")
}

fn base_url_payload() -> serde_json::Value {
    serde_json::json!({
        "owner": "acme",
        "repo": "bpmux",
        "base_url": "http://127.0.0.1:9012/"
    })
}

#[tokio::test]
async fn get_reports_disconnected_on_a_fresh_database() {
    let h = harness();
    let response = send(h.router, "GET", None, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["state"], "disconnected");
}

#[tokio::test]
async fn get_reports_the_connected_repo_and_never_the_token() {
    let h = harness();
    h.connections
        .save(&GithubConnection::Hosted {
            owner: "RyanDur".to_string(),
            repo: "bpmux".to_string(),
            token: "ghp_secret".to_string(),
        })
        .expect("save");
    let response = send(h.router, "GET", None, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["state"], "connected");
    assert_eq!(body["owner"], "RyanDur");
    assert_eq!(body["repo"], "bpmux");
    assert!(
        !body.to_string().contains("ghp_secret"),
        "the token never crosses the status endpoint"
    );
}

#[tokio::test]
async fn get_reports_a_vanished_token_as_disconnected() {
    let h = harness();
    h.connections
        .save(&GithubConnection::Hosted {
            owner: "RyanDur".to_string(),
            repo: "bpmux".to_string(),
            token: "ghp_secret".to_string(),
        })
        .expect("save");
    h.secrets.delete(GITHUB_TOKEN_KEY).expect("delete");
    let response = send(h.router, "GET", None, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body_json(response).await["state"],
        "disconnected",
        "graceful: a half-present connection is simply not a connection — reconnect from the UI"
    );
}

#[tokio::test]
async fn put_without_the_owner_nonce_is_refused_once_claimed() {
    let h = harness();
    claim_owner_nonce(&h, "launch-1");
    let response = send(h.router.clone(), "PUT", Some(base_url_payload()), None).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let stale = send(
        h.router,
        "PUT",
        Some(base_url_payload()),
        Some("stale-nonce"),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        h.connections.resolve().expect("resolve"),
        None,
        "a refused connect persists nothing"
    );
}

#[tokio::test]
async fn put_before_any_claim_proceeds_as_the_bootstrap_fallback() {
    let h = harness();
    let response = send(h.router, "PUT", Some(base_url_payload()), None).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "same bootstrap semantic as board acceptance: before any launch nonce is claimed, the walk proceeds"
    );
}

#[tokio::test]
async fn put_persists_what_it_is_given_and_re_resolves_the_actions_url() {
    let h = harness();
    claim_owner_nonce(&h, "launch-1");
    let response = send(h.router, "PUT", Some(base_url_payload()), Some("launch-1")).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["state"], "connected");
    assert_eq!(body["owner"], "acme");
    assert!(
        matches!(
            h.connections.resolve().expect("resolve"),
            Some(GithubConnection::WithBaseUrl { .. })
        ),
        "the door stores the connection as given — it proves itself in use, gracefully"
    );
    assert_eq!(
        h.actions_url.current(),
        "https://github.com/acme/bpmux/actions",
        "connecting re-resolves the live actions url without a restart"
    );
}

#[tokio::test]
async fn put_switching_to_a_base_url_connection_clears_the_stale_token() {
    let h = harness();
    h.connections
        .save(&GithubConnection::Hosted {
            owner: "RyanDur".to_string(),
            repo: "bpmux".to_string(),
            token: "ghp_old".to_string(),
        })
        .expect("save hosted");
    claim_owner_nonce(&h, "launch-1");
    let response = send(h.router, "PUT", Some(base_url_payload()), Some("launch-1")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        h.secrets.load(GITHUB_TOKEN_KEY).expect("load"),
        None,
        "switching routes through disconnect-then-save — no orphaned token in the keychain"
    );
}

#[tokio::test]
async fn put_reconfigures_in_place_and_overwrites_the_stored_credential() {
    let h = harness();
    h.connections
        .save(&GithubConnection::Hosted {
            owner: "RyanDur".to_string(),
            repo: "bpmux".to_string(),
            token: "ghp_old".to_string(),
        })
        .expect("save first connection");
    claim_owner_nonce(&h, "launch-1");
    let response = send(
        h.router,
        "PUT",
        Some(serde_json::json!({
            "owner": "RyanDur",
            "repo": "sand",
            "token": "ghp_new"
        })),
        Some("launch-1"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        h.secrets.load(GITHUB_TOKEN_KEY).expect("load"),
        Some("ghp_new".to_string()),
        "reconnecting overwrites the credential — the old token is gone, not shadowed"
    );
    assert_eq!(
        h.connections.resolve().expect("resolve"),
        Some(GithubConnection::Hosted {
            owner: "RyanDur".to_string(),
            repo: "sand".to_string(),
            token: "ghp_new".to_string(),
        }),
        "new values replace the old wholesale — one connection, always the latest"
    );
    assert_eq!(
        h.actions_url.current(),
        "https://github.com/RyanDur/sand/actions",
        "the live actions url follows the reconfiguration"
    );
}

#[tokio::test]
async fn put_with_neither_token_nor_base_url_is_refused() {
    let h = harness();
    claim_owner_nonce(&h, "launch-1");
    let response = send(
        h.router,
        "PUT",
        Some(serde_json::json!({"owner": "acme", "repo": "bpmux"})),
        Some("launch-1"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(h.connections.resolve().expect("resolve"), None);
}

#[tokio::test]
async fn delete_disconnects_and_resets_the_actions_url() {
    let h = harness();
    h.connections
        .save(&GithubConnection::WithBaseUrl {
            owner: "acme".to_string(),
            repo: "bpmux".to_string(),
            base_url: "http://127.0.0.1:9012/".to_string(),
        })
        .expect("save");
    h.actions_url.set_repo(Some(("acme", "bpmux")));
    claim_owner_nonce(&h, "launch-1");
    let response = send(h.router, "DELETE", None, Some("launch-1")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["state"], "disconnected");
    assert_eq!(h.connections.resolve().expect("resolve"), None);
    assert_eq!(h.actions_url.current(), "https://github.com");
}

#[tokio::test]
async fn delete_without_the_owner_nonce_is_refused_once_claimed() {
    let h = harness();
    h.connections
        .save(&GithubConnection::WithBaseUrl {
            owner: "acme".to_string(),
            repo: "bpmux".to_string(),
            base_url: "http://127.0.0.1:9012/".to_string(),
        })
        .expect("save");
    claim_owner_nonce(&h, "launch-1");
    let response = send(h.router, "DELETE", None, None).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        h.connections.resolve().expect("resolve").is_some(),
        "a refused disconnect changes nothing"
    );
}
