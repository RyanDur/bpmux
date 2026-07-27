use std::sync::Arc;

pub(crate) struct GithubBoot {
    pub connections: transport::rest::ConnectionsHandle,
    pub actions_url: ci::ActionsUrl,
    pub tracker: transport::rest::TrackerHandle,
}

pub(crate) fn resolve(data_gateways: &storage::DataGateways) -> GithubBoot {
    let secrets: Arc<dyn integrations::github::connection::SecretStore> =
        if super::setup::ephemeral_secrets_enabled() {
            Arc::new(integrations::in_memory::InMemorySecretStore::default())
        } else {
            Arc::new(integrations::github::keychain::KeychainSecretStore)
        };
    let connections: transport::rest::ConnectionsHandle = Arc::new(
        integrations::data_gateway::GithubConnectionDataGateway::new(
            data_gateways.gateway.clone(),
            secrets,
        ),
    );
    let connection = match connections.resolve() {
        Ok(connection) => connection,
        Err(e) => {
            diag::log_diag(&format!(
                "github-connection: resolve failed at boot — running unconnected: {e}"
            ));
            None
        }
    };
    let actions_url = ci::ActionsUrl::for_repo(
        connection
            .as_ref()
            .map(integrations::github::connection::GithubConnection::owner_repo),
    );
    let tracker: transport::rest::TrackerHandle = connection
        .as_ref()
        .and_then(integrations::github::GitHubSourceAdapter::from_connection)
        .map(Arc::new);
    GithubBoot {
        connections,
        actions_url,
        tracker,
    }
}
