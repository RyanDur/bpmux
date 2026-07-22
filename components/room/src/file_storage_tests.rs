#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::file_storage::*;
use crate::{Room, RoomStore};
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

#[test]
fn list_returns_empty_vec_when_base_dir_is_absent() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().join("never-created"));
    assert_eq!(storage.list().unwrap(), vec![]);
}

#[test]
fn list_returns_empty_vec_when_base_dir_has_no_rooms() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().to_path_buf());
    assert_eq!(storage.list().unwrap(), vec![]);
}

#[test]
fn list_enumerates_every_saved_room() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().to_path_buf());
    storage.save(&fixture("pm")).unwrap();
    storage.save(&fixture("dev")).unwrap();

    let mut roles: Vec<String> = storage
        .list()
        .unwrap()
        .into_iter()
        .map(|r| r.role)
        .collect();
    roles.sort();
    assert_eq!(roles, vec!["dev".to_string(), "pm".to_string()]);
}

#[test]
fn read_errors_when_room_is_not_spawned() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().to_path_buf());
    let err = storage.read("ghost", None).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert!(err.to_string().contains("ghost"));
}

#[test]
fn read_returns_empty_when_spawned_but_no_output_captured_yet() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().to_path_buf());
    storage.save(&fixture("pm")).unwrap();
    assert_eq!(storage.read("pm", None).unwrap(), "");
}

#[test]
fn read_returns_the_full_captured_output_by_default() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().to_path_buf());
    storage.save(&fixture("pm")).unwrap();
    std::fs::write(storage.output_path("pm"), "line1\nline2\nline3\n").unwrap();
    assert_eq!(storage.read("pm", None).unwrap(), "line1\nline2\nline3\n");
}

#[test]
fn read_returns_only_the_last_n_lines_when_bounded() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().to_path_buf());
    storage.save(&fixture("pm")).unwrap();
    std::fs::write(storage.output_path("pm"), "l1\nl2\nl3\nl4\nl5\n").unwrap();
    assert_eq!(storage.read("pm", Some(2)).unwrap(), "l4\nl5");
}

#[test]
fn get_returns_the_saved_room() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().to_path_buf());
    let room = fixture("pm");
    storage.save(&room).unwrap();
    assert_eq!(storage.get("pm").unwrap(), room);
}

#[test]
fn get_errors_when_room_is_not_spawned() {
    let dir = tempdir().unwrap();
    let storage = FileRoomStorage::new(dir.path().to_path_buf());
    let err = storage.get("ghost").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert!(err.to_string().contains("ghost"));
}
