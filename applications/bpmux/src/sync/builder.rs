use super::*;
use std::sync::Arc;

use crate::sync::SourceAdapter;

impl SyncOrchestrator {
    pub fn new<A: SourceAdapter + 'static>(
        adapter: Arc<A>,
        codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
        select: crate::store::Select,
        dispatch: crate::store::Dispatch,
    ) -> Self {
        Self {
            dispatch,
            adapter,
            codec,
            select,
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            drain_guard: tokio::sync::Mutex::new(()),
        }
    }
}
