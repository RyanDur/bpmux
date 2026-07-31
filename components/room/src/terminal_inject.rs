const MARKER: &str = "⟦bpmux⟧";
const SEPARATOR: char = '▸';
const FRAMING_STYLE: &str = "\x1b[2;36m";
const STYLE_RESET: &str = "\x1b[0m";

pub fn frame_team_message(sender: &str, scope: &str, body: &str) -> String {
    let sender = sanitize_field(sender);
    let scope = sanitize_field(scope);
    let body = sanitize_field(body);
    format!("{FRAMING_STYLE}{MARKER} {sender} {scope}{SEPARATOR}{STYLE_RESET} {body}")
}

fn sanitize_field(field: &str) -> String {
    let mut stripped = String::with_capacity(field.len());
    let mut chars = field.chars().peekable();
    while let Some(current) = chars.next() {
        if current == '\x1b' {
            skip_escape_sequence(&mut chars);
            continue;
        }
        if !current.is_control() {
            stripped.push(current);
        }
    }
    while stripped.contains(MARKER) {
        stripped = stripped.replace(MARKER, "");
    }
    stripped
}

fn skip_escape_sequence(chars: &mut std::iter::Peekable<std::str::Chars>) {
    if chars.peek() != Some(&'[') {
        if chars.peek().is_some() {
            chars.next();
        }
        return;
    }
    chars.next();
    while let Some(&following) = chars.peek() {
        chars.next();
        if following.is_ascii_alphabetic() {
            break;
        }
    }
}

use message::{BROADCAST_ALL, Message};
use user_routing::short_name;

pub(crate) fn framed_line(msg: &Message) -> String {
    let scope = scope_label(msg.to.as_deref(), msg.to_ids.as_deref());
    frame_team_message(short_name(&msg.from), &scope, &msg.text)
}

fn scope_label(to: Option<&[String]>, to_ids: Option<&[String]>) -> String {
    if to.is_none() && to_ids.is_some() {
        return "direct".to_string();
    }
    match to {
        None => "broadcast".to_string(),
        Some(names) if names.iter().any(|name| name == BROADCAST_ALL) => "broadcast".to_string(),
        Some(names) if names.len() == 1 => "→you".to_string(),
        Some(names) => format!(
            "to:{}",
            names
                .iter()
                .map(|name| short_name(name))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}
