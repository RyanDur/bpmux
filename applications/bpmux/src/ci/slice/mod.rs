use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::hub::Hub;
use kernel::Dispatch;
use rate_limit::RateLimitObserver;
use store::AppAction;

pub use ::ci::slice::{
    BoxedFuture, CiAction, CiBroadcastListener, CiPollFetch, CiSnapshot, CiState, hydrated,
    last_successful_poll_at, last_updated_at, polling, reduce, report_from_state, snapshot,
    snapshot_key,
};

pub trait CiWatchDriver: Send + Sync {
    fn spawn_watch(
        &self,
        dispatch: Dispatch<AppAction>,
        tracker: Arc<dyn CiPollFetch>,
        hub: Arc<Hub>,
        observer: RateLimitObserver,
        ct: CancellationToken,
    );
}

mod poll;

pub use poll::CiPoll;
