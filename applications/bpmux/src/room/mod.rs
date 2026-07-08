use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod agnostic_delivery;
pub mod file_storage;
pub mod live_registry;
pub mod spawn;
mod terminal_inject;
pub mod wake_delivery;

#[cfg(test)]
mod agnostic_delivery_tests;
#[cfg(test)]
mod file_storage_tests;
#[cfg(test)]
mod live_registry_tests;
#[cfg(test)]
mod room_helpers;
#[cfg(test)]
mod terminal_injection;

pub use agnostic_delivery::{deliver_to_terminals, drain_missed, spawn_terminal_delivery_loop};
pub use file_storage::FileRoomStorage;
pub use live_registry::LivePtyRegistry;
pub use spawn::{
    BridgeWiring, LivePty, Osc7Integration, osc7_shell_integration, resolve_terminal_spawn,
    spawn_agent_run_detector, spawn_captured, spawn_pty, spawn_pty_retained, terminal_provisioning,
    terminate,
};
pub use wake_delivery::wake_committed_terminals;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    pub role: String,
    pub agent: String,
    pub pid: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub trait RoomStore {
    fn save(&self, room: &Room) -> std::io::Result<()>;

    fn list(&self) -> std::io::Result<Vec<Room>>;

    fn read(&self, role: &str, lines: Option<usize>) -> std::io::Result<String>;

    fn get(&self, role: &str) -> std::io::Result<Room>;
}

pub fn strip_ansi(input: &str) -> String {
    String::from_utf8_lossy(&strip_ansi_escapes::strip(input)).into_owned()
}

pub fn rooms_dir() -> PathBuf {
    resolve_rooms_dir(
        std::env::var_os("BPMUX_ROOMS_DIR"),
        std::env::var_os("HOME"),
    )
}

fn resolve_rooms_dir(
    env_override: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> PathBuf {
    if let Some(p) = env_override {
        return PathBuf::from(p);
    }
    home.map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".bpmux")
        .join("rooms")
}
