mod backend;
mod data;
mod instrumented;

pub use backend::open;
pub use data::*;

pub trait Storage:
    awareness::SurfacedTopicsRepo
    + message_log::MessageLog
    + memory::lessons::LessonRepo
    + source_sync::OutboxRepo
    + user_routing::UserRepo
    + Send
    + Sync
{
    fn load_state(&self, key: &str) -> Option<String>;
    fn save_state(&self, key: &str, value: &str);
}

#[cfg(test)]
mod backend_tests;
#[cfg(test)]
mod boundary_tests;
