use crate::sync::{MutationId, QueuedWrite};

#[derive(Clone, Debug)]
pub enum SyncAction {
    StartIngestPolling,
    StopIngestPolling,
    StartHydrationPolling,
    StopHydrationPolling,
    WriteQueued { write: QueuedWrite },
    WritesQueued(Vec<QueuedWrite>),
    WritePushed { id: MutationId },
    WriteDropped { id: MutationId },
}
