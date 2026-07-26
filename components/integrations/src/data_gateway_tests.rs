#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use crate::data_gateway::GithubConnectionDataGateway;
use crate::github::connection::{GITHUB_TOKEN_KEY, GithubConnection, SecretStore};
use crate::in_memory::InMemorySecretStore;
use database_support::DatabaseGateway;

struct Harness {
    connections: GithubConnectionDataGateway,
    secrets: Arc<InMemorySecretStore>,
    db: Arc<DatabaseGateway>,
    _dir: tempfile::TempDir,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = Arc::new(DatabaseGateway::open(dir.path()).expect("open temp database"));
    let secrets = Arc::new(InMemorySecretStore::default());
    Harness {
        connections: GithubConnectionDataGateway::new(db.clone(), secrets.clone()),
        secrets,
        db,
        _dir: dir,
    }
}

fn hosted(owner: &str, repo: &str, token: &str) -> GithubConnection {
    GithubConnection::Hosted {
        owner: owner.to_string(),
        repo: repo.to_string(),
        token: token.to_string(),
    }
}

#[test]
fn resolve_on_a_fresh_database_is_none() {
    let h = harness();
    assert_eq!(h.connections.resolve().expect("resolve"), None);
}

#[test]
fn a_hosted_connection_round_trips_with_its_token_from_the_secret_store() {
    let h = harness();
    h.connections
        .save(&hosted("RyanDur", "bpmux", "ghp_secret"))
        .expect("save");
    assert_eq!(
        h.connections.resolve().expect("resolve"),
        Some(hosted("RyanDur", "bpmux", "ghp_secret"))
    );
    assert_eq!(
        h.secrets.load(GITHUB_TOKEN_KEY).expect("load"),
        Some("ghp_secret".to_string()),
        "the token lives in the secret store, keyed for the keychain"
    );
}

#[test]
fn a_base_url_connection_round_trips_without_touching_the_secret_store() {
    let h = harness();
    let mock = GithubConnection::WithBaseUrl {
        owner: "acme".to_string(),
        repo: "bpmux".to_string(),
        base_url: "http://127.0.0.1:9012".to_string(),
    };
    h.connections.save(&mock).expect("save");
    assert_eq!(h.connections.resolve().expect("resolve"), Some(mock));
    assert_eq!(
        h.secrets.touches(),
        0,
        "base_url connections are token-less — the secret store is never consulted"
    );
}

#[test]
fn saving_a_new_connection_replaces_the_old() {
    let h = harness();
    h.connections
        .save(&hosted("RyanDur", "bpmux", "ghp_old"))
        .expect("save first");
    h.connections
        .save(&hosted("RyanDur", "sand", "ghp_new"))
        .expect("save second");
    assert_eq!(
        h.connections.resolve().expect("resolve"),
        Some(hosted("RyanDur", "sand", "ghp_new")),
        "one active connection — reconnect replaces, never accumulates"
    );
}

#[test]
fn disconnect_clears_both_the_row_and_the_token() {
    let h = harness();
    h.connections
        .save(&hosted("RyanDur", "bpmux", "ghp_secret"))
        .expect("save");
    h.connections.disconnect().expect("disconnect");
    assert_eq!(h.connections.resolve().expect("resolve"), None);
    assert_eq!(h.secrets.load(GITHUB_TOKEN_KEY).expect("load"), None);
}

#[test]
fn disconnect_on_a_fresh_database_is_a_no_op() {
    let h = harness();
    h.connections.disconnect().expect("disconnect");
    assert_eq!(h.connections.resolve().expect("resolve"), None);
}

#[test]
fn a_hosted_row_whose_token_vanished_resolves_as_unconnected() {
    let h = harness();
    h.connections
        .save(&hosted("RyanDur", "bpmux", "ghp_secret"))
        .expect("save");
    h.secrets.delete(GITHUB_TOKEN_KEY).expect("delete");
    assert_eq!(
        h.connections.resolve().expect("resolve"),
        None,
        "graceful: a half-present connection is simply not a connection — reconnect from the UI"
    );
}

#[test]
fn the_schema_holds_no_token_column() {
    let h = harness();
    let columns: Vec<String> = database_support::query(
        &h.db.lock(),
        "SELECT name FROM pragma_table_info('github_connection') ORDER BY cid",
        [],
        |row| row.get(0),
    )
    .expect("table info");
    assert_eq!(
        columns,
        vec!["id", "owner", "repo", "base_url"],
        "the token never lands in the database — hosted connections keep it in the OS keychain"
    );
}
