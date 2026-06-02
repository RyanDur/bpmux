use std::collections::HashMap;
use std::sync::Mutex;

use super::spawn::LivePty;

#[derive(Default)]
pub struct LivePtyRegistry {
    rooms: Mutex<HashMap<String, LivePty>>,
}

impl LivePtyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, role: String, live: LivePty) {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role, live);
    }

    pub fn contains(&self, role: &str) -> bool {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(role)
    }

    pub fn reader_for(&self, role: &str) -> Option<std::io::Result<Box<dyn std::io::Read + Send>>> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .map(|live| live.reader())
    }

    pub fn write_to(&self, role: &str, bytes: &[u8]) -> bool {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .map(|live| live.write_input(bytes).is_ok())
            .unwrap_or(false)
    }

    pub fn resize(&self, terminal_id: &str, rows: u16, cols: u16) -> bool {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(terminal_id)
            .map(|live| live.resize(rows, cols).is_ok())
            .unwrap_or(false)
    }

    pub fn remove(&self, terminal_id: &str) -> Option<LivePty> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(terminal_id)
    }

    pub fn has_foreground_job(&self, terminal_id: &str) -> Option<bool> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(terminal_id)
            .map(|live| live.has_foreground_job())
    }

    pub fn reap_dead_and_list_live(&self) -> Vec<String> {
        let mut rooms = self.rooms.lock().unwrap_or_else(|e| e.into_inner());
        let mut live = Vec::new();
        rooms.retain(|terminal_id, pty| {
            if pty.is_alive() {
                live.push(terminal_id.clone());
                true
            } else {
                false
            }
        });
        live
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::room::spawn::spawn_pty_retained;
    use std::io::Read;

    #[test]
    fn registry_hands_out_a_working_reader_keyed_by_role() {
        let registry = LivePtyRegistry::new();
        let live = spawn_pty_retained(
            "/bin/sh",
            &["-c".into(), "printf 'registry-stream-marker'".into()],
            None,
            &[],
        )
        .unwrap();
        registry.insert("dev".to_string(), live);

        assert!(registry.contains("dev"));
        assert!(!registry.contains("pm"));

        let mut reader = registry
            .reader_for("dev")
            .expect("dev room present")
            .unwrap();
        let mut collected = String::new();
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if collected.contains("registry-stream-marker") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        assert!(
            collected.contains("registry-stream-marker"),
            "registry.reader_for must stream the role's live PTY output; got {collected:?}"
        );
    }

    #[test]
    fn reader_for_absent_role_is_none() {
        let registry = LivePtyRegistry::new();
        assert!(registry.reader_for("nobody").is_none());
    }

    #[test]
    fn registry_write_reaches_the_same_live_session_stdin_across_multiple_writes() {
        let registry = LivePtyRegistry::new();
        let live = spawn_pty_retained(
            "/bin/sh",
            &["-c".into(), "read line; printf 'GOT:%s' \"$line\"".into()],
            None,
            &[],
        )
        .unwrap();
        registry.insert("dev".to_string(), live);

        assert!(registry.write_to("dev", b"ping"));
        assert!(
            registry.write_to("dev", b"-stdin\n"),
            "the SECOND write must also reach stdin — the writer is taken once at spawn and reused, not re-taken per write"
        );

        let mut reader = registry.reader_for("dev").expect("dev present").unwrap();
        let mut collected = String::new();
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if collected.contains("GOT:ping-stdin") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        assert!(
            collected.contains("GOT:ping-stdin"),
            "writing to the room write-half must reach the SAME live session's stdin (the child reads it and echoes GOT:<line> on the same master); got {collected:?}"
        );
    }

    #[test]
    fn write_to_absent_role_is_false() {
        let registry = LivePtyRegistry::new();
        assert!(!registry.write_to("nobody", b"x"));
    }

    fn drain_room_until(
        reader: Option<std::io::Result<Box<dyn std::io::Read + Send>>>,
        marker: &str,
    ) -> String {
        use std::io::Read;
        let mut reader = reader.expect("room present").unwrap();
        let mut collected = String::new();
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if collected.contains(marker) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        collected
    }

    #[test]
    fn two_rooms_are_isolated_no_cross_bleed_and_both_coexist() {
        let registry = LivePtyRegistry::new();
        let dev = spawn_pty_retained(
            "/bin/sh",
            &["-c".into(), "printf 'DEV-ROOM-OUTPUT'".into()],
            None,
            &[],
        )
        .unwrap();
        let pm = spawn_pty_retained(
            "/bin/sh",
            &["-c".into(), "printf 'PM-ROOM-OUTPUT'".into()],
            None,
            &[],
        )
        .unwrap();
        registry.insert("dev".to_string(), dev);
        registry.insert("pm".to_string(), pm);

        assert!(
            registry.contains("dev") && registry.contains("pm"),
            "spawning the pm room must not destabilize the dev room — both coexist"
        );

        let dev_out = drain_room_until(registry.reader_for("dev"), "DEV-ROOM-OUTPUT");
        let pm_out = drain_room_until(registry.reader_for("pm"), "PM-ROOM-OUTPUT");

        assert!(dev_out.contains("DEV-ROOM-OUTPUT"));
        assert!(
            !dev_out.contains("PM-ROOM-OUTPUT"),
            "EXERCISED bleed-vector: pm's output must NOT reach the dev room; got {dev_out:?}"
        );
        assert!(pm_out.contains("PM-ROOM-OUTPUT"));
        assert!(
            !pm_out.contains("DEV-ROOM-OUTPUT"),
            "EXERCISED bleed-vector: dev's output must NOT reach the pm room; got {pm_out:?}"
        );
    }

    #[test]
    fn reap_dead_and_list_live_returns_only_living_ids_and_reaps_the_exited_out_of_the_registry() {
        let registry = LivePtyRegistry::new();
        let living =
            spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[])
                .unwrap();
        let exiting =
            spawn_pty_retained("/bin/sh", &["-c".into(), "exit 0".into()], None, &[]).unwrap();
        registry.insert("living-terminal".to_string(), living);
        registry.insert("exited-terminal".to_string(), exiting);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let live = loop {
            let live = registry.reap_dead_and_list_live();
            if !live.iter().any(|id| id == "exited-terminal") {
                break live;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the exited terminal never dropped out of the live set"
            );
            std::thread::sleep(std::time::Duration::from_millis(25));
        };

        assert!(
            live.iter().any(|id| id == "living-terminal"),
            "the still-running terminal must remain in the live set; got {live:?}"
        );
        assert!(
            registry.contains("living-terminal"),
            "reconciling must not evict a live terminal from the registry"
        );
        assert!(
            !registry.contains("exited-terminal"),
            "the exited terminal must be reaped OUT of the registry — that single pass closes the orphan-PTY leak (d) AND prevents the exited terminal resurrecting on the next refresh's re-attach (e)"
        );
    }
}
