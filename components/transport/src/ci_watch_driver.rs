use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use hub::Hub;
use kernel::Dispatch;
use rate_limit::RateLimitObserver;
use store::ci_watch::CiWatchDriver;
use store::{AppAction, CiPollFetch};

use crate::ci_watcher::spawn;

pub type CiFetchResolver = Arc<dyn Fn() -> Option<Arc<dyn CiPollFetch>> + Send + Sync>;

pub fn cell_fetch_resolver(cell: crate::rest::shared::TrackerCell) -> CiFetchResolver {
    Arc::new(move || {
        cell.current()
            .map(|adapter| adapter as Arc<dyn CiPollFetch>)
    })
}

pub struct CiWatcherDriver {
    pub actions_url: ci::ActionsUrl,
    pub fetch: CiFetchResolver,
}

impl CiWatchDriver for CiWatcherDriver {
    fn spawn_watch(
        &self,
        dispatch: Dispatch<AppAction>,
        hub: Arc<Hub>,
        observer: RateLimitObserver,
        ct: CancellationToken,
    ) {
        spawn(
            dispatch,
            self.fetch.clone(),
            hub,
            observer,
            ct,
            self.actions_url.clone(),
        );
    }
}
