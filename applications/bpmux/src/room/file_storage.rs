use std::path::PathBuf;

use super::{Room, RoomStore};

pub struct FileRoomStorage {
    base: PathBuf,
}

impl FileRoomStorage {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    pub fn from_env() -> Self {
        Self::new(super::rooms_dir())
    }

    pub fn config_path(&self, role: &str) -> PathBuf {
        self.base.join(role).join("config.json")
    }
}

impl RoomStore for FileRoomStorage {
    fn save(&self, room: &Room) -> std::io::Result<()> {
        let dir = self.base.join(&room.role);
        std::fs::create_dir_all(&dir)?;
        let body = serde_json::to_string_pretty(room)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let path = dir.join("config.json");
        std::fs::write(&path, body)?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fixture(role: &str) -> Room {
        Room {
            role: role.to_string(),
            agent: "claude-code".to_string(),
            pid: 42,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn save_creates_role_subdir_and_config_json() {
        let dir = tempdir().unwrap();
        let storage = FileRoomStorage::new(dir.path().to_path_buf());
        storage.save(&fixture("pm")).unwrap();
        let cfg = dir.path().join("pm").join("config.json");
        assert!(cfg.exists(), "config.json must land under <base>/<role>/");
    }

    #[test]
    fn save_round_trips_through_config_json() {
        let dir = tempdir().unwrap();
        let storage = FileRoomStorage::new(dir.path().to_path_buf());
        let room = fixture("dev");
        storage.save(&room).unwrap();
        let body = std::fs::read_to_string(storage.config_path("dev")).unwrap();
        let parsed: Room = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed, room);
    }

    #[test]
    fn save_overwrites_existing_config_for_same_role() {
        let dir = tempdir().unwrap();
        let storage = FileRoomStorage::new(dir.path().to_path_buf());
        let mut room = fixture("designer");
        storage.save(&room).unwrap();
        room.pid = 99;
        storage.save(&room).unwrap();
        let body = std::fs::read_to_string(storage.config_path("designer")).unwrap();
        let parsed: Room = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed.pid, 99);
    }
}
