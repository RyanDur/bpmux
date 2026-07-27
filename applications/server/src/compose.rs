use std::sync::Arc;

use store::Store;
use sync::SyncOrchestrator;
use tokio_util::sync::CancellationToken;

use super::store_wiring::build_store_with_refresh;
pub(crate) fn compose_store(
    hub: Arc<hub::Hub>,
    data_gateways: storage::DataGateways,
    cell: &integrations::github::live::TrackerCell,
    connected: bridge::polling::ConnectedProbe,
    ct: CancellationToken,
    actions_url: ci::ActionsUrl,
) -> (Store, Arc<SyncOrchestrator>) {
    let hydrated_outbox = super::setup::hydrate_outbox(&data_gateways.outbox);
    let codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync> =
        Arc::new(source_codec::gh_label_codec::GhLabelCodec);

    if super::setup::test_mode_enabled() {
        eprintln!(
            "[bpmux server] BPMUX_TEST_MODE=1 — instantiating in-memory orchestrator for acceptance tests"
        );
        let adapter = Arc::new(integrations::in_memory::InMemorySourceAdapter::new());
        build_store_with_refresh(
            hub,
            data_gateways,
            cell.clone(),
            connected,
            ct,
            hydrated_outbox,
            adapter,
            codec,
            bridge::poll_interval_from_env(),
            bridge::hydration_interval_from_env(),
            actions_url,
        )
    } else {
        diag::log_diag(&format!(
            "server: board-outbox hydrated via storage ({} pending events)",
            hydrated_outbox.len()
        ));
        let adapter = Arc::new(integrations::github::live::SwappableSource::new(
            cell.clone(),
        ));
        build_store_with_refresh(
            hub,
            data_gateways,
            cell.clone(),
            connected,
            ct,
            hydrated_outbox,
            adapter,
            codec,
            bridge::poll_interval_from_env(),
            bridge::hydration_interval_from_env(),
            actions_url,
        )
    }
}
