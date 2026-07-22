use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::AppAction;
use hub::Hub;
use kernel::Dispatch;
use rate_limit::RateLimitObserver;

use ci::slice::{CiAction, CiPollFetch};

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
