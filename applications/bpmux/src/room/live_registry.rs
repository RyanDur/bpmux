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

    pub fn writer_for(
        &self,
        role: &str,
    ) -> Option<std::io::Result<Box<dyn std::io::Write + Send>>> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .map(|live| live.writer())
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
    fn registry_write_reaches_the_same_live_session_stdin() {
        use std::io::Write;
        let registry = LivePtyRegistry::new();
        let live = spawn_pty_retained(
            "/bin/sh",
            &["-c".into(), "read line; printf 'GOT:%s' \"$line\"".into()],
        )
        .unwrap();
        registry.insert("dev".to_string(), live);

        let mut writer = registry
            .writer_for("dev")
            .expect("dev room present")
            .unwrap();
        writer.write_all(b"ping-stdin\n").unwrap();
        writer.flush().unwrap();

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
    fn writer_for_absent_role_is_none() {
        let registry = LivePtyRegistry::new();
        assert!(registry.writer_for("nobody").is_none());
    }
}
