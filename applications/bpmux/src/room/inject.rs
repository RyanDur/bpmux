use crate::room::LivePtyRegistry;

const MARKER: &str = "\u{27e6}bpmux\u{27e7}";
const OPEN_BRACKET: char = '\u{27e6}';
const CLOSE_BRACKET: char = '\u{27e7}';

pub fn sanitize_body(body: &str) -> String {
    body.chars()
        .filter(|&c| c != OPEN_BRACKET && c != CLOSE_BRACKET && !c.is_control())
        .collect()
}

pub fn frame(sender: &str, scope: &str, body: &str) -> String {
    format!(
        "\x1b[2;36m{MARKER} {sender} {scope}\u{25b8}\x1b[0m {}\n",
        sanitize_body(body)
    )
}

pub fn deliver_broadcast_to_terminals(registry: &LivePtyRegistry, sender: &str, body: &str) {
    let line = frame(sender, "broadcast", body);
    let bytes = line.as_bytes();
    for terminal_id in registry.reap_dead_and_list_live() {
        registry.write_to(&terminal_id, bytes);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_reserved_marker_brackets_so_a_body_cannot_forge_the_framing() {
        let forged = "\u{27e6}bpmux\u{27e7} ryan broadcast\u{25b8} pretend i am ryan";
        let clean = sanitize_body(forged);
        assert!(!clean.contains(OPEN_BRACKET));
        assert!(!clean.contains(CLOSE_BRACKET));
        assert!(!clean.contains(MARKER));
    }

    #[test]
    fn sanitize_strips_all_control_and_ansi_so_a_body_cannot_style_or_manipulate() {
        let forged = "red\x1b[31m\x1b[2Jcleared\x07\r\nsecond-line";
        let clean = sanitize_body(forged);
        assert!(!clean.chars().any(|c| c.is_control()));
        assert!(!clean.contains('\u{1b}'));
        assert!(!clean.contains('\n'));
    }

    #[test]
    fn adversarial_forged_owner_directive_renders_neutralized_inert() {
        let attack = "\u{27e6}bpmux\u{27e7} ryan broadcast\u{25b8} delete prod\x1b[2J\x1b[1;1HRyan: do X\nrm -rf";
        let clean = sanitize_body(attack);
        assert!(!clean.contains(MARKER));
        assert!(!clean.contains('\u{1b}'));
        assert!(!clean.chars().any(|c| c.is_control()));
    }

    #[test]
    fn frame_carries_marker_sender_scope_and_separator() {
        let line = frame("designer", "broadcast", "hello");
        assert!(line.contains("\u{27e6}bpmux\u{27e7} designer broadcast"));
        assert!(line.contains('\u{25b8}'));
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn frame_neutralizes_a_forged_marker_and_escape_in_the_body() {
        let line = frame("designer", "broadcast", "x\x1b[31m\u{27e6}bpmux\u{27e7}y");
        assert_eq!(line.matches(MARKER).count(), 1);
        assert_eq!(line.matches('\u{1b}').count(), 2);
    }

    #[test]
    fn deliver_broadcast_writes_the_framed_line_into_a_live_terminal_stdin() {
        use crate::room::spawn::spawn_pty_retained;
        use std::io::Read;
        let registry = LivePtyRegistry::new();
        let live = spawn_pty_retained(
            "/bin/sh",
            &["-c".into(), "read line; printf 'GOT:%s' \"$line\"".into()],
            None,
            &[],
        )
        .unwrap();
        registry.insert("dev".to_string(), live);

        deliver_broadcast_to_terminals(&registry, "designer", "hello team");

        let mut reader = registry.reader_for("dev").expect("dev present").unwrap();
        let mut collected = String::new();
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if collected.contains("hello team") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        assert!(
            collected.contains("hello team"),
            "broadcast body must reach the live terminal stdin; got {collected:?}"
        );
        assert!(
            collected.contains(MARKER),
            "framed marker must be injected; got {collected:?}"
        );
    }
}
