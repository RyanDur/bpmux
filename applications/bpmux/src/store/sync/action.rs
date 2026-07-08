use crate::sync::MutationId;

#[derive(Clone, Debug)]
pub enum SyncAction {
    StartIngestPolling,
    StopIngestPolling,
    StartHydrationPolling,
    StopHydrationPolling,
    StartUpdateGithub { mutation_id: MutationId },
}
