use super::*;
use std::sync::Arc;

use crate::sync::SourceAdapter;

impl SyncOrchestrator {
    pub fn new<A: SourceAdapter + 'static>(
        adapter: Arc<A>,
        codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    ) -> Self {
        Self {
            dispatch: std::sync::OnceLock::new(),
            adapter,
            codec,
            select: std::sync::OnceLock::new(),
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            drain_guard: tokio::sync::Mutex::new(()),
        }
    }
}
