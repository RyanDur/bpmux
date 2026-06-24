use super::GlobalStore;

mod file_global;
mod memory_global;

pub use file_global::FileGlobalStore;
pub use memory_global::MemoryGlobalStore;
