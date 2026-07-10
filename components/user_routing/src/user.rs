#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserRecord {
    #[serde(alias = "session_id")]
    pub user_id: String,
    pub name: String,
    pub role: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub last_delivered_at: Option<i64>,
}

pub fn default_kind() -> String {
    "agent".to_string()
}

pub trait UserRepo: Send + Sync {
    fn load_users(&self) -> Result<Vec<UserRecord>, persist::PersistError>;
    fn upsert_user(&self, id: &str, name: &str, role: &str) -> Result<(), persist::PersistError>;
    fn set_user_last_delivered(&self, id: &str, at: i64) -> Result<(), persist::PersistError>;
    fn delete_users(&self, ids: &[String]) -> Result<(), persist::PersistError>;
}
