use super::*;
use std::sync::Arc;

use crate::SourceAdapter;

impl SyncOrchestrator {
    pub fn new<A: SourceAdapter + 'static>(
        adapter: Arc<A>,
        codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
        select: store::Select,
        dispatch: store::Dispatch,
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
