use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::ci::CiStateStore;
use crate::hub::Hub;
use crate::store::AppAction;
use kernel::Dispatch;
use rate_limit::RateLimitObserver;

pub use ::ci::slice::{
    BoxedFuture, CiAction, CiBroadcastListener, CiPollFetch, CiSnapshot, CiState,
    CiStateStoreWriter, last_successful_poll_at, last_updated_at, polling, reduce, snapshot,
    snapshot_key,
};

pub trait CiWatchDriver: Send + Sync {
    fn spawn_watch(
        &self,
        dispatch: Dispatch<AppAction>,
        tracker: Arc<dyn CiPollFetch>,
        ci_state_store: CiStateStore,
        hub: Arc<Hub>,
        observer: RateLimitObserver,
        ct: CancellationToken,
    );
}

mod poll;

pub use poll::CiPoll;
