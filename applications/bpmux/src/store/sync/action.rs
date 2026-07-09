use crate::sync::{MutationId, PendingWrite};

#[derive(Clone, Debug)]
pub enum SyncAction {
    StartIngestPolling,
    StopIngestPolling,
    StartHydrationPolling,
    StopHydrationPolling,
    WriteQueued { write: PendingWrite },
    WritesQueued(Vec<PendingWrite>),
    WritePushed { id: MutationId },
    WriteDropped { id: MutationId },
}
