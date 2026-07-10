use super::FileStorage;
use crate::routing::{UserRecord, UserRepo, default_kind};
use persist::PersistError;

impl UserRepo for FileStorage {
    fn load_users(&self) -> Result<Vec<UserRecord>, PersistError> {
        let path = self.dir.join("users.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = std::fs::read(&path)?;
        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn upsert_user(&self, id: &str, name: &str, role: &str) -> Result<(), PersistError> {
        let mut records = self.load_users()?;
        match records.iter_mut().find(|r| r.user_id == id) {
            Some(existing) => {
                existing.name = name.to_string();
                existing.role = role.to_string();
            }
            None => records.push(UserRecord {
                user_id: id.to_string(),
                name: name.to_string(),
                role: role.to_string(),
                kind: default_kind(),
                last_delivered_at: None,
            }),
        }
        self.write_users(&records)
    }

    fn set_user_last_delivered(&self, id: &str, at: i64) -> Result<(), PersistError> {
        let mut records = self.load_users()?;
        if let Some(existing) = records.iter_mut().find(|r| r.user_id == id) {
            existing.last_delivered_at = Some(at);
        }
        self.write_users(&records)
    }

    fn delete_users(&self, ids: &[String]) -> Result<(), PersistError> {
        let mut records = self.load_users()?;
        records.retain(|r| !ids.iter().any(|id| id == &r.user_id));
        self.write_users(&records)
    }
}
