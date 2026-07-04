use crate::sync::MutationId;

#[derive(Clone, Debug)]
pub enum SyncAction {
    StartIngestPolling,
    StopIngestPolling,
    StartHydrationPolling,
    StopHydrationPolling,
    StartUpdateGithub {
        mutation_id: MutationId,
    },
    StopUpdateGithub {
        mutation_id: MutationId,
    },
    MutationConfirmed {
        mutation_id: MutationId,
    },
    MutationRolledBack {
        mutation_id: MutationId,
        server_story: Option<Box<crate::product_management::story::Story>>,
    },
}
