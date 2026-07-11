mod backend;
mod data;
mod instrumented;

pub use backend::StorageBackend;
pub use data::*;

pub trait Storage:
    crate::awareness::SurfacedTopicsRepo
    + message_log::MessageLog
    + memory::lessons::LessonRepo
    + crate::sync::OutboxRepo
    + user_routing::UserRepo
    + Send
    + Sync
{
    fn load_state(&self, key: &str) -> Option<String>;
    fn save_state(&self, key: &str, value: &str);
}

#[cfg(test)]
mod boundary_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) fn sqlite_test_storage(dir: &std::path::Path) -> std::sync::Arc<dyn Storage> {
    std::sync::Arc::new(SqliteStorage::new(dir.to_path_buf()).expect("sqlite test storage"))
}
