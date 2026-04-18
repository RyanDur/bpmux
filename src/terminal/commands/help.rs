//! `/help` slash command + shared command-list formatter (#164).
//!
//! Extracted from `commands.rs` to keep that file under the 400-line
//! soft limit. The formatter is shared by:
//!
//! - `/help` chat-pane output (`handle_help` here)
//! - The `?` keybind help overlay (`ui::render_help_overlay`)
//!
//! Both consumers source from `input::SLASH_COMMANDS` so the chat-pane
//! and overlay forms can never drift — adding a command means adding
//! one line to that constant and nothing else.

use crate::session::Message;
use crate::terminal::app::App;
use crate::terminal::input::{HELP_DOMAINS, SLASH_COMMANDS, SlashCommand};

impl App {
    /// `/help` (#164) — print the slash-command list inline in the chat
    /// pane. Sources from `SLASH_COMMANDS` so this list and the `?`
    /// overlay can never drift again. Uses the single-column form
    /// because chat-pane width is generous and full descriptions are
    /// the whole point of `/help` (vs. the overlay's compact form).
    pub(in crate::terminal) fn handle_help(&mut self) -> Option<Message> {
        self.system_message(format_command_list(1));
        self.clear_input();
        None
    }
}

/// Build the slash-command list text from `SLASH_COMMANDS`. Used by
/// `/help` (chat-pane) and `?` keybind (overlay) so they can never drift
/// (#164).
///
/// `columns` controls the layout: `1` produces a single column with the
/// full (long) description per command — the chat-pane form, where width
/// is generous. `2` produces a compact two-column form with short
/// descriptions — the overlay form, where width is tight but names alone
/// aren't enough to teach the user what each command does. Both forms
/// include descriptions; only the LENGTH of the description varies by
/// surface. This was the load-bearing fix in the #164 re-delivery — the
/// original shipped `2` form dropped descriptions entirely, which Ryan
/// correctly rejected as a regression.
/// Collect commands belonging to `domain`, alphabetized by name.
/// Returns an empty Vec when no command carries the domain — the
/// caller is responsible for skipping empty groups so retirement of
/// the last command in a domain doesn't leave a header pointing at
/// nothing (AC #284: empty groups don't render).
fn commands_in_domain(domain: &str) -> Vec<&'static SlashCommand> {
    let mut in_domain: Vec<&'static SlashCommand> = SLASH_COMMANDS
        .iter()
        .filter(|c| c.domain == domain)
        .collect();
    in_domain.sort_by_key(|c| c.name);
    in_domain
}

pub(crate) fn format_command_list(columns: usize) -> String {
    let mut out = String::from("Commands:\n");
    let widest_name = SLASH_COMMANDS
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(0);
    if columns <= 1 {
        // Single-column chat-pane form: full long descriptions, grouped
        // by semantic domain (#284). Renders a dim-prefix header per
        // group; empty groups skipped so commands retiring don't leave
        // dangling headers.
        for domain in HELP_DOMAINS {
            let entries = commands_in_domain(domain);
            if entries.is_empty() {
                continue;
            }
            out.push_str(&format!("  ──── {domain} ────\n"));
            for c in entries {
                let name = c.name;
                let long = c.long_desc;
                out.push_str(&format!("  {name:<widest_name$}  {long}\n"));
            }
        }
    } else {
        // Two-column compact overlay form grouped by domain (#284).
        // Each domain lays out its own two-column block so the user
        // sees the group header once, then the commands in that group
        // fill columns before the next group begins. Empty groups are
        // skipped.
        let widest_short = SLASH_COMMANDS
            .iter()
            .map(|c| c.short_desc.len())
            .max()
            .unwrap_or(0);
        let entry_width = widest_name + 1 + widest_short;
        let gap = 2;
        for domain in HELP_DOMAINS {
            let entries = commands_in_domain(domain);
            if entries.is_empty() {
                continue;
            }
            out.push_str(&format!("  ──── {domain} ────\n"));
            for chunk in entries.chunks(columns) {
                out.push_str("  ");
                for (i, c) in chunk.iter().enumerate() {
                    let name = c.name;
                    let short = c.short_desc;
                    let entry = format!("{name:<widest_name$} {short:<widest_short$}");
                    if i + 1 < chunk.len() {
                        out.push_str(&format!("{entry:<width$}", width = entry_width + gap));
                    } else {
                        out.push_str(&entry);
                    }
                }
                out.push('\n');
            }
        }
    }
    // Strip the trailing newline so the system_message doesn't render
    // an empty line at the bottom.
    out.pop();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────
    // `format_command_list` unit tests.
    //
    // These are UNIT TESTS on the pure `format_command_list` function.
    // They call the function directly with a specific `columns` value
    // and assert on its return string. They do NOT exercise the real
    // `?` overlay render path — that's covered end-to-end by
    // `tests/story60_acceptance_test.rs::ac5_help_overlay_shows_command_descriptions`.
    //
    // Keep both layers: the unit tests here are cheap regression guards
    // on the function's return value; the story60 test exercises the
    // real path from keypress through render. Each is appropriate for
    // its scope.
    //
    // **Rename audit (2026-04-11 evening)**: originally named
    // `overlay_form_includes_short_descriptions_beside_names` and
    // `chat_pane_form_still_carries_long_descriptions`, framing unit
    // tests as overlay/chat-pane BEHAVIOR assertions. Ryan caught the
    // mismatch — the names promised system behavior, the content was
    // pure function output assertions. Renamed to match content.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn format_command_list_2col_output_contains_short_descriptions() {
        // Guards the fix from the #164 re-delivery: the 2-column branch
        // must include short descriptions, not just command names.
        // If this starts failing, someone has reverted the dropping-
        // descriptions regression Ryan rejected.
        let output = format_command_list(2);
        assert!(output.contains("/to"));
        assert!(output.contains("/share"));
        // Description tokens chosen to avoid collision with command
        // names: "propagate" appears only in /share's desc, "toggle"
        // in /all and /debug descs, "filter" in /all's desc. None
        // appear as slash-command names themselves.
        let has_description =
            output.contains("propagate") || output.contains("toggle") || output.contains("filter");
        assert!(
            has_description,
            "expected at least one of: propagate, toggle, filter in output:\n{output}"
        );
    }

    #[test]
    fn format_command_list_1col_output_contains_long_descriptions() {
        // Guards that the chat-pane form (columns=1) uses the LONG
        // description from the 3-tuple, not the short. If this fails,
        // someone has swapped columns and the chat-pane /help output
        // is now using the compressed short form instead of the full
        // text.
        let output = format_command_list(1);
        assert!(output.contains("/to"));
        // "send to recipients" is the long description for /to.
        // The short description is "send to roles" — distinct text.
        assert!(
            output.contains("send to recipients"),
            "1-col output should contain /to's long description; got:\n{output}"
        );
    }

    // ─────────────────────────────────────────────────────────────────
    // Domain-grouping tests (#284).
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn format_command_list_1col_renders_domain_headers_in_order() {
        // AC: /help renders View, Compose, Filter, Team state, Meta
        // in that order. Check each header appears and the indices are
        // monotonically increasing.
        let output = format_command_list(1);
        let positions: Vec<Option<usize>> = HELP_DOMAINS
            .iter()
            .map(|d| output.find(&format!("──── {d} ────")))
            .collect();
        for (i, p) in positions.iter().enumerate() {
            assert!(
                p.is_some(),
                "domain header {} not found in 1-col output:\n{output}",
                HELP_DOMAINS[i]
            );
        }
        // Each subsequent header must appear after the previous one.
        let flat: Vec<usize> = positions.into_iter().map(|p| p.unwrap_or(0)).collect();
        for i in 1..flat.len() {
            assert!(
                flat[i] > flat[i - 1],
                "domain headers out of order: {:?} in\n{output}",
                HELP_DOMAINS
            );
        }
    }

    #[test]
    fn format_command_list_2col_renders_domain_headers_in_order() {
        // AC: `?` overlay carries the same grouping.
        let output = format_command_list(2);
        let positions: Vec<Option<usize>> = HELP_DOMAINS
            .iter()
            .map(|d| output.find(&format!("──── {d} ────")))
            .collect();
        for (i, p) in positions.iter().enumerate() {
            assert!(
                p.is_some(),
                "domain header {} not found in 2-col output:\n{output}",
                HELP_DOMAINS[i]
            );
        }
        let flat: Vec<usize> = positions.into_iter().map(|p| p.unwrap_or(0)).collect();
        for i in 1..flat.len() {
            assert!(
                flat[i] > flat[i - 1],
                "domain headers out of order: {:?} in\n{output}",
                HELP_DOMAINS
            );
        }
    }

    #[test]
    fn commands_within_domain_sort_alphabetically() {
        // AC: within each group, alphabetical order. Meta has 7 entries
        // today so it's the strictest alphabetical check.
        let meta = commands_in_domain("Meta");
        let names: Vec<&str> = meta.iter().map(|c| c.name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(
            names, sorted,
            "Meta commands should be alphabetically ordered, got {names:?}"
        );
    }

    #[test]
    fn empty_domain_does_not_render_header() {
        // AC: empty groups don't render. Exercise commands_in_domain
        // directly with a domain that has zero registered commands to
        // confirm the grouping filter returns empty — format_command_list
        // uses the same filter, so an empty result guarantees no header.
        let empty = commands_in_domain("NonExistentDomain");
        assert!(
            empty.is_empty(),
            "non-existent domain should yield empty commands vec"
        );
    }

    #[test]
    fn every_registered_command_lands_in_a_known_domain() {
        // Guard: every SLASH_COMMANDS entry's domain string matches
        // one of HELP_DOMAINS. Prevents typos or mis-routing at
        // registration time (a new command with domain: "View " with
        // trailing space would silently fall out of all groups and
        // become invisible in /help).
        for c in SLASH_COMMANDS {
            assert!(
                HELP_DOMAINS.contains(&c.domain),
                "command {} has domain {:?} which is not in HELP_DOMAINS; \
                 either fix the domain or add the domain to HELP_DOMAINS",
                c.name,
                c.domain
            );
        }
    }

    #[test]
    fn format_command_list_1col_contains_every_registered_command() {
        // Regression guard: no command disappears from /help output as
        // a side effect of grouping. Every SLASH_COMMANDS entry must
        // still appear by name.
        let output = format_command_list(1);
        for c in SLASH_COMMANDS {
            assert!(
                output.contains(c.name),
                "command {} missing from 1-col output:\n{output}",
                c.name
            );
        }
    }
}
