use super::InstrumentedStorage;
use crate::storage::Storage;

impl<S: Storage> crate::awareness::SurfacedTopicsRepo for InstrumentedStorage<S> {
    fn load_surfaced_topics(&self) -> std::collections::HashSet<String> {
        self.inner.load_surfaced_topics()
    }

    fn save_surfaced_topics(
        &self,
        topics: &std::collections::HashSet<String>,
    ) -> Result<(), std::io::Error> {
        self.inner.save_surfaced_topics(topics)
    }
}
