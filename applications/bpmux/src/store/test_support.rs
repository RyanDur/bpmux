#![allow(clippy::panic)]

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

pub(crate) fn wired_store(
    hub: Arc<crate::hub::Hub>,
    tracker: Option<Arc<dyn crate::store::CiPollFetch>>,
    parent_ct: CancellationToken,
) -> crate::store::Store {
    crate::server::store_wiring::build_store(hub, tracker, parent_ct)
}

pub(crate) use store_test_support::eventually;
