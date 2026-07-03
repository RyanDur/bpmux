#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserRecord {
    pub session_id: String,
    pub name: String,
    pub role: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub last_delivered_at: Option<i64>,
}

pub(crate) fn default_kind() -> String {
    "agent".to_string()
}

pub trait UserRepo: Send + Sync {
    fn load_users(&self) -> Result<Vec<UserRecord>, crate::persist::PersistError>;
    fn upsert_user(
        &self,
        id: &str,
        name: &str,
        role: &str,
    ) -> Result<(), crate::persist::PersistError>;
    fn set_user_last_delivered(
        &self,
        id: &str,
        at: i64,
    ) -> Result<(), crate::persist::PersistError>;
    fn delete_users(&self, ids: &[String]) -> Result<(), crate::persist::PersistError>;
}
