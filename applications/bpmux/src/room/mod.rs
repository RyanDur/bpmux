use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod agnostic_delivery;
pub mod commitment_store;
pub mod file_storage;
pub mod live_registry;
pub mod spawn;
pub mod wake_delivery;

#[cfg(test)]
mod agnostic_delivery_tests;
#[cfg(test)]
mod commitment_store_tests;
#[cfg(test)]
mod file_storage_tests;
#[cfg(test)]
mod live_registry_tests;

pub use agnostic_delivery::{deliver_to_terminals, drain_missed, spawn_terminal_delivery_loop};
pub use commitment_store::CommitmentStore;
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn resolve_falls_back_to_home_dotbpmux_rooms_when_env_unset() {
        let dir = resolve_rooms_dir(None, Some("/home/agent".into()));
        assert_eq!(dir, PathBuf::from("/home/agent/.bpmux/rooms"));
    }

    #[test]
    fn resolve_honours_env_override() {
        let dir = resolve_rooms_dir(
            Some("/tmp/bpmux-rooms-test".into()),
            Some("/ignored".into()),
        );
        assert_eq!(dir, PathBuf::from("/tmp/bpmux-rooms-test"));
    }

    #[test]
    fn resolve_falls_back_to_tmp_when_both_unset() {
        let dir = resolve_rooms_dir(None, None);
        assert_eq!(dir, PathBuf::from("/tmp/.bpmux/rooms"));
    }

    #[test]
    fn strip_ansi_removes_color_escape_sequences() {
        let raw = "\x1b[31mred\x1b[0m plain";
        assert_eq!(strip_ansi(raw), "red plain");
        assert!(!strip_ansi(raw).contains('\u{1b}'));
    }

    #[test]
    fn strip_ansi_leaves_clean_text_untouched() {
        assert_eq!(strip_ansi("just text\nline two"), "just text\nline two");
    }

    #[test]
    fn room_round_trips_through_json() {
        let room = Room {
            role: "pm".to_string(),
            agent: "claude-code".to_string(),
            pid: 12345,
            created_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&room).unwrap();
        let parsed: Room = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, room);
    }
}
