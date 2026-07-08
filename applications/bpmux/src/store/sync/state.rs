use std::collections::VecDeque;

use crate::sync::QueuedWrite;

#[derive(Default, Clone)]
pub struct SyncState {
    pub queue: VecDeque<QueuedWrite>,
}
