pub(super) const MARKER: &str = "⟦bpmux⟧";
pub(super) const SEPARATOR: char = '▸';
pub(super) const FRAMING_STYLE: &str = "\x1b[2;36m";
pub(super) const STYLE_RESET: &str = "\x1b[0m";

pub fn frame_team_message(sender: &str, scope: &str, body: &str) -> String {
    let sender = sanitize_field(sender);
    let scope = sanitize_field(scope);
    let body = sanitize_field(body);
    format!("{FRAMING_STYLE}{MARKER} {sender} {scope}{SEPARATOR}{STYLE_RESET} {body}")
}

fn sanitize_field(field: &str) -> String {
    let mut stripped = String::with_capacity(field.len());
    let mut chars = field.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&following) = chars.peek() {
                    chars.next();
                    if following.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if chars.peek().is_some() {
                chars.next();
            }
            continue;
        }
        if !c.is_control() {
            stripped.push(c);
        }
    }
    while stripped.contains(MARKER) {
        stripped = stripped.replace(MARKER, "");
    }
    stripped
}
