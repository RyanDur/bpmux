use std::collections::VecDeque;

use crate::QueuedWrite;

#[derive(Clone)]
pub struct SyncState {
    pub queue: VecDeque<QueuedWrite>,
    pub next_mutation_id: u64,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            next_mutation_id: 1,
        }
    }
}
