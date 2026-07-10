use std::time::Instant;

use super::InstrumentedStorage;
use crate::storage::Storage;

impl<S: Storage> user_routing::UserRepo for InstrumentedStorage<S> {
    fn load_users(&self) -> Result<Vec<user_routing::UserRecord>, persist::PersistError> {
        let start = Instant::now();
        let result = self.inner.load_users();
        if let Ok(ref records) = result {
            self.log(&format!(
                "load_users {} users {:.1}ms",
                records.len(),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        result
    }

    fn upsert_user(&self, id: &str, name: &str, role: &str) -> Result<(), persist::PersistError> {
        let start = Instant::now();
        let result = self.inner.upsert_user(id, name, role);
        self.log(&format!(
            "upsert_user '{}' {:.1}ms",
            id,
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn set_user_last_delivered(&self, id: &str, at: i64) -> Result<(), persist::PersistError> {
        let start = Instant::now();
        let result = self.inner.set_user_last_delivered(id, at);
        self.log(&format!(
            "set_user_last_delivered '{}' {:.1}ms",
            id,
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn delete_users(&self, ids: &[String]) -> Result<(), persist::PersistError> {
        let start = Instant::now();
        let result = self.inner.delete_users(ids);
        self.log(&format!(
            "delete_users {} ids {:.1}ms",
            ids.len(),
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }
}
