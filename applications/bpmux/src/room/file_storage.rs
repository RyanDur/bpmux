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

    pub fn output_path(&self, role: &str) -> PathBuf {
        self.base.join(role).join("output.log")
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

    fn list(&self) -> std::io::Result<Vec<Room>> {
        let entries = match std::fs::read_dir(&self.base) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut rooms = Vec::new();
        for entry in entries {
            let config = entry?.path().join("config.json");
            if !config.is_file() {
                continue;
            }
            let body = std::fs::read_to_string(&config)?;
            let room: Room = serde_json::from_str(&body)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            rooms.push(room);
        }
        Ok(rooms)
    }

    fn read(&self, role: &str, lines: Option<usize>) -> std::io::Result<String> {
        if !self.config_path(role).is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("room '{role}' is not spawned"),
            ));
        }
        let body = match std::fs::read_to_string(self.output_path(role)) {
            Ok(body) => body,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
            Err(e) => return Err(e),
        };
        match lines {
            Some(n) => {
                let all: Vec<&str> = body.lines().collect();
                let start = all.len().saturating_sub(n);
                Ok(all[start..].join("\n"))
            }
            None => Ok(body),
        }
    }

    fn get(&self, role: &str) -> std::io::Result<Room> {
        let body = match std::fs::read_to_string(self.config_path(role)) {
            Ok(body) => body,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("room '{role}' is not spawned"),
                ));
            }
            Err(e) => return Err(e),
        };
        serde_json::from_str(&body)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
