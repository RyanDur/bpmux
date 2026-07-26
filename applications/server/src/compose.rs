use std::sync::Arc;

use store::{CiPollFetch, Store};
use sync::SyncOrchestrator;
use tokio_util::sync::CancellationToken;

use super::store_wiring::{build_store_with_outbox, build_store_with_refresh};
pub(crate) fn compose_store(
    hub: Arc<hub::Hub>,
    data_gateways: storage::DataGateways,
    tracker: &transport::rest::TrackerHandle,
    ct: CancellationToken,
) -> (Store, Option<Arc<SyncOrchestrator>>) {
    let hydrated_outbox = super::setup::hydrate_outbox(&data_gateways.outbox);
    let ci_tracker = tracker.as_ref().cloned().map(|t| t as Arc<dyn CiPollFetch>);
    let codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync> =
        Arc::new(source_codec::gh_label_codec::GhLabelCodec);

    if super::setup::test_mode_enabled() {
        eprintln!(
            "[bpmux server] BPMUX_TEST_MODE=1 — instantiating in-memory orchestrator for acceptance tests"
        );
        let adapter = Arc::new(integrations::in_memory::InMemorySourceAdapter::new());
        let (store, refresh) = build_store_with_refresh(
            hub,
            data_gateways,
            ci_tracker,
            ct,
            hydrated_outbox,
            adapter,
            codec,
            bridge::poll_interval_from_env(),
            bridge::hydration_interval_from_env(),
        );
        (store, Some(refresh))
    } else if let Some(tracker_arc) = tracker.as_ref().cloned() {
        diag::log_diag(&format!(
            "server: board-outbox hydrated via storage ({} pending events)",
            hydrated_outbox.len()
        ));
        let (store, refresh) = build_store_with_refresh(
            hub,
            data_gateways,
            ci_tracker,
            ct,
            hydrated_outbox,
            tracker_arc,
            codec,
            bridge::poll_interval_from_env(),
            bridge::hydration_interval_from_env(),
        );
        (store, Some(refresh))
    } else {
        let store = build_store_with_outbox(hub, data_gateways, ci_tracker, ct, hydrated_outbox);
        (store, None)
    }
}
