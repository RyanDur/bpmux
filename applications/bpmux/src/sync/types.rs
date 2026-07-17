use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::SyncOrchestrator;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type OrchestratorHandle = Option<Arc<SyncOrchestrator>>;
