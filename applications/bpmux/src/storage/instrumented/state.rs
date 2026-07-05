use super::InstrumentedStorage;
use crate::storage::Storage;

impl<S: Storage> Storage for InstrumentedStorage<S> {
    fn load_state(&self, key: &str) -> Option<String> {
        self.inner.load_state(key)
    }

    fn save_state(&self, key: &str, value: &str) {
        self.inner.save_state(key, value)
    }
}
