//! `/focus` slash command + focus-filter helper (#181).
//!
//! Part A of the two-part focus mode: a client-side display filter
//! that narrows the chat viewport to messages from a specified set
//! of roles. Part B (agent-side attention routing) is a separate
//! shaping story.
//!
//! Extracted into its own module to keep `commands.rs` under its
//! 400-line soft limit, matching the `commands_tokens.rs` /
//! `commands_help.rs` / `commands_share.rs` precedent.

use crate::session::{Message, MessageType};
use crate::terminal::app::App;

/// Parsed `/focus` argument shapes. Kept as an enum rather than
/// direct mutation so the handler can log a clear error for
/// invalid input without writing partial state.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum FocusAction {
    /// Activate the filter for a specific set of roles.
    Set(Vec<String>),
    /// Deactivate the filter (show everything).
    Off,
    /// Report the current filter state without changing it.
    Status,
}

/// Parse `/focus` args into a `FocusAction`. Pure function.
///
/// Accepted shapes (case-insensitive on keywords, case-preserving
/// on role names since roles are identifiers):
///
/// - `""`     — no args, treated as `Status`
/// - `"off"`  — clear the filter
/// - `"all"`  — alias for `off`
/// - `"status"` — explicit status query
/// - `"dev,pm"` — set filter to `[dev, pm]`
/// - `"dev, pm, designer"` — whitespace tolerant
///
/// Empty roles after splitting (e.g. `"dev,,pm"`) are dropped.
pub(super) fn parse_focus_args(args: &str) -> FocusAction {
    let trimmed = args.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
        return FocusAction::Status;
    }
    if trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case(crate::session::message::BROADCAST_ALL)
    {
        return FocusAction::Off;
    }
    let roles: Vec<String> = trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if roles.is_empty() {
        FocusAction::Off
    } else {
        FocusAction::Set(roles)
    }
}

/// Is `msg` visible under the current `focused` role set, from the
/// point of view of `my_role` (the local user — typically Ryan).
/// Pure function. Called from `chat::render_chat` per message.
///
/// Rules (per #181 AC):
/// - If no filter (`focused == None`), everything is visible.
/// - The user's own messages are always visible (the
///   "you see what you just sent" property).
/// - System messages are always visible (connection events, etc.).
/// - Messages whose `addressee` is the user's role bypass the filter
///   — someone addressing you directly should not be hidden even if
///   their role isn't in your focus set.
/// - Otherwise, show only if the sender's role (the prefix before
///   `:` in `msg.from`) is in the `focused` set.
pub(crate) fn in_focus(msg: &Message, focused: Option<&[String]>, my_role: &str) -> bool {
    let Some(focused) = focused else {
        return true;
    };
    if msg.message_type == MessageType::System {
        return true;
    }
    let sender_role = msg.from.split(':').next().unwrap_or(&msg.from);
    if sender_role == my_role {
        return true;
    }
    if msg.addressee.as_deref() == Some(my_role) {
        return true;
    }
    focused.iter().any(|r| r == sender_role)
}

impl App {
    /// `/focus <args>` handler. Parses args, mutates `App::focused_roles`,
    /// emits a system message describing the new state.
    pub(in crate::terminal) fn handle_focus(&mut self, args: String) -> Option<Message> {
        match parse_focus_args(&args) {
            FocusAction::Set(roles) => {
                // Known = peers seen in messages + peer_activity + self
                let mut known: std::collections::HashSet<String> =
                    self.peers.activity.keys().cloned().collect();
                known.insert(self.identity.name.clone());
                for m in &self.msg.messages {
                    let sender = m.from.split(':').next().unwrap_or(&m.from);
                    if sender != "bpmux" {
                        known.insert(sender.to_string());
                    }
                }
                let known: Vec<String> = {
                    let mut v: Vec<String> = known.into_iter().collect();
                    v.sort();
                    v
                };
                let mut valid = Vec::new();
                let mut unknown = Vec::new();
                for role in &roles {
                    if known.iter().any(|k| k == role) {
                        if !valid.contains(role) {
                            valid.push(role.clone());
                        }
                    } else {
                        unknown.push(role.clone());
                    }
                }
                for u in &unknown {
                    let known_list = known.join(", ");
                    self.system_message(format!("Unknown role \"{u}\". Known: {known_list}."));
                }
                if valid.is_empty() {
                    self.system_message("Focus unchanged.".to_string());
                } else {
                    let joined = valid.join(", ");
                    self.focused_roles = Some(valid);
                    self.mark_filters_dirty();
                    self.system_message(format!("Focus: {joined}."));
                }
            }
            FocusAction::Off => {
                self.focused_roles = None;
                self.mark_filters_dirty();
                self.system_message("Focus cleared.".to_string());
            }
            FocusAction::Status => {
                let text = match self.focused_roles.as_ref() {
                    None => "Focus: off.".to_string(),
                    Some(roles) => format!("Focus: {}.", roles.join(", ")),
                };
                self.system_message(text);
            }
        }
        self.clear_input();
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn msg(from: &str, text: &str) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            ..Message::default()
        }
    }

    fn system_msg(from: &str, text: &str) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: MessageType::System,
            ..Message::default()
        }
    }

    // ── parse_focus_args ──────────────────────────────────────────

    #[test]
    fn parse_focus_args_empty_input_is_status() {
        assert_eq!(parse_focus_args(""), FocusAction::Status);
        assert_eq!(parse_focus_args("   "), FocusAction::Status);
        assert_eq!(parse_focus_args("status"), FocusAction::Status);
        assert_eq!(parse_focus_args("STATUS"), FocusAction::Status);
    }

    #[test]
    fn parse_focus_args_off_and_all_clear_the_filter() {
        assert_eq!(parse_focus_args("off"), FocusAction::Off);
        assert_eq!(parse_focus_args("OFF"), FocusAction::Off);
        assert_eq!(parse_focus_args("all"), FocusAction::Off);
        assert_eq!(parse_focus_args("All"), FocusAction::Off);
    }

    #[test]
    fn parse_focus_args_comma_list_is_set() {
        assert_eq!(
            parse_focus_args("dev,pm"),
            FocusAction::Set(vec!["dev".to_string(), "pm".to_string()])
        );
    }

    #[test]
    fn parse_focus_args_trims_whitespace_around_role_names() {
        assert_eq!(
            parse_focus_args("dev, pm, designer"),
            FocusAction::Set(vec![
                "dev".to_string(),
                "pm".to_string(),
                "designer".to_string()
            ])
        );
    }

    #[test]
    fn parse_focus_args_drops_empty_entries_from_consecutive_commas() {
        assert_eq!(
            parse_focus_args("dev,,pm"),
            FocusAction::Set(vec!["dev".to_string(), "pm".to_string()])
        );
    }

    #[test]
    fn parse_focus_args_all_empty_entries_collapses_to_off() {
        assert_eq!(parse_focus_args(",,,"), FocusAction::Off);
    }

    // ── in_focus ──────────────────────────────────────────────────

    #[test]
    fn in_focus_returns_true_when_no_filter_is_active() {
        let m = msg("designer:claude-code", "hello");
        assert!(in_focus(&m, None, "ryan"));
    }

    #[test]
    fn in_focus_returns_true_when_sender_role_is_in_focused_set() {
        let m = msg("dev:claude-code", "hello");
        assert!(in_focus(
            &m,
            Some(&["dev".to_string(), "pm".to_string()]),
            "ryan"
        ));
    }

    #[test]
    fn in_focus_returns_false_when_sender_role_is_not_in_focused_set() {
        let m = msg("designer:claude-code", "hello");
        assert!(!in_focus(
            &m,
            Some(&["dev".to_string(), "pm".to_string()]),
            "ryan"
        ));
    }

    #[test]
    fn in_focus_always_shows_messages_from_my_own_role() {
        let m = msg("ryan:human", "what i just typed");
        assert!(in_focus(
            &m,
            Some(&["dev".to_string()]), // only dev in focus
            "ryan"
        ));
    }

    #[test]
    fn in_focus_always_shows_system_messages() {
        let m = system_msg("bpmux", "session started");
        assert!(in_focus(
            &m,
            Some(&["dev".to_string()]), // only dev in focus
            "ryan"
        ));
    }

    #[test]
    fn in_focus_shows_filtered_out_role_when_addressee_is_me() {
        let mut m = msg("designer:claude-code", "ryan, look at this");
        m.addressee = Some("ryan".to_string());
        assert!(in_focus(
            &m,
            Some(&["dev".to_string()]), // designer not in focus
            "ryan"
        ));
    }

    #[test]
    fn in_focus_does_not_leak_when_addressee_is_a_different_role() {
        // An addressee field targeting someone else does NOT bypass
        // the filter for me. The override is only for my_role.
        let mut m = msg("designer:claude-code", "pm, check this");
        m.addressee = Some("pm".to_string());
        assert!(!in_focus(&m, Some(&["dev".to_string()]), "ryan"));
    }

    #[test]
    fn in_focus_handles_bare_role_names_without_harness_suffix() {
        // Legacy or bridge-minted senders may lack the `role:harness`
        // split. The sender-role parse should fall back to the whole
        // `from` string.
        let m = msg("bpmux", "activity");
        assert!(in_focus(&m, Some(&["bpmux".to_string()]), "ryan"));
        assert!(!in_focus(&m, Some(&["dev".to_string()]), "ryan"));
    }
}
