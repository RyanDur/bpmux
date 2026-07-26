#![allow(clippy::panic)]

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

pub(crate) fn wired_store(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn store::CiPollFetch>>,
    parent_ct: CancellationToken,
) -> store::Store {
    let data_gateways = storage::open(&hub.dir);
    crate::store_wiring::build_store(hub, data_gateways, tracker, parent_ct)
}

pub(crate) use store_test_support::eventually;
