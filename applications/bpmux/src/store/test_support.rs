#![allow(clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

pub(crate) fn wired_store(
    hub: Arc<crate::hub::Hub>,
    tracker: Option<Arc<dyn crate::store::CiPollFetch>>,
    ci_state_store: crate::ci::CiStateStore,
    parent_ct: CancellationToken,
) -> crate::store::Store {
    crate::server::store_wiring::build_store(hub, tracker, ci_state_store, parent_ct)
}

pub(crate) async fn eventually(label: &str, mut condition: impl FnMut() -> bool) {
    for _ in 0..400 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("eventually({label}): condition not satisfied within 2s");
}
