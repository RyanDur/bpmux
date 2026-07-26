use std::sync::Arc;

use crate::{UserRecord, UserRepo, db};
use database_support::{DatabaseGateway, persist_err};
use persist::PersistError;

pub struct UserStore {
    gateway: Arc<DatabaseGateway>,
}

impl UserStore {
    pub fn new(gateway: Arc<DatabaseGateway>) -> Self {
        Self { gateway }
    }
}

impl UserRepo for UserStore {
    fn load_users(&self) -> Result<Vec<UserRecord>, PersistError> {
        db::load_all(&self.gateway.lock()).map_err(persist_err)
    }

    fn upsert_user(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError> {
        db::upsert(&self.gateway.lock(), id, name, role)
            .map(|_| ())
            .map_err(persist_err)
    }

    fn set_user_last_delivered(&self, id: &str, at: i64) -> Result<(), PersistError> {
        db::set_last_delivered(&self.gateway.lock(), id, at)
            .map(|_| ())
            .map_err(persist_err)
    }

    fn delete_users(&self, ids: &[String]) -> Result<(), PersistError> {
        db::delete(&self.gateway.lock(), ids)
            .map(|_| ())
            .map_err(persist_err)
    }
}
