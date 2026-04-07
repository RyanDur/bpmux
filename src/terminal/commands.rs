use super::app::App;
use crate::session::Message;
use chrono::Utc;

pub(super) const MAX_HISTORY: usize = 50;

impl App {
    pub fn submit_message(&mut self) -> Option<Message> {
        self.dm_unread_from.clear();
        self.dm_flash_until = None;
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }

        if text == "/who" {
            return self.handle_who();
        }
        if text == "/quit" {
            return self.handle_quit();
        }
        if text == "/debug" {
            return self.handle_debug();
        }
        if text.starts_with("/status ") {
            return self.handle_status(text);
        }
        if let Some(rest) = text.strip_prefix("/to ") {
            return self.handle_to_with_args(rest.to_string());
        }
        if text == "/to" {
            return self.handle_to_bare();
        }
        if text == "/view" {
            return self.handle_view_bare();
        }
        if let Some(arg) = text.strip_prefix("/view ") {
            return self.handle_view_with_arg(arg.to_string());
        }
        if text == "/pins" {
            return self.handle_pins();
        }
        if let Some(arg) = text.strip_prefix("/pin ") {
            return self.handle_pin_with_arg(arg.to_string());
        }
        if text == "/pin" {
            return self.handle_pin_bare();
        }
        if let Some(arg) = text.strip_prefix("/unpin ") {
            return self.handle_unpin_with_arg(arg.to_string());
        }
        if text == "/unpin" {
            return self.handle_unpin_bare();
        }
        if text == "/board" {
            return self.handle_board();
        }
        if text == "/roadmap" {
            return self.handle_roadmap();
        }
        if text.starts_with('/') {
            return self.handle_unknown_command(text);
        }

        self.send_chat_message(text)
    }

    fn handle_who(&mut self) -> Option<Message> {
        let mut participants = vec![format!("{} (you)", self.peer_name)];
        for (peer, (status, _)) in &self.peer_activity {
            participants.push(format!("{peer} [{status}]"));
        }
        if self.connected_peers > 0 {
            participants.push(format!("{} connected", self.connected_peers));
        }
        self.system_message(format!("Participants:\n  {}", participants.join("\n  ")));
        self.clear_input();
        None
    }

    fn handle_quit(&mut self) -> Option<Message> {
        self.should_quit = true;
        None
    }

    fn handle_debug(&mut self) -> Option<Message> {
        self.show_system_messages = !self.show_system_messages;
        let state = if self.show_system_messages {
            "on"
        } else {
            "off"
        };
        self.system_message(format!(
            "Debug mode {state}. System messages are now {}.",
            if self.show_system_messages {
                "visible"
            } else {
                "hidden"
            }
        ));
        self.clear_input();
        None
    }

    fn handle_status(&mut self, text: String) -> Option<Message> {
        // Send as a regular message — the convention is parsed by receivers
        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text,
            ts: Utc::now(),
            message_type: None,
            to: None,
        };
        self.messages.push(msg.clone());
        self.clear_input();
        self.scroll_offset = 0;
        self.unread_count = 0;
        Some(msg)
    }

    fn handle_to_with_args(&mut self, rest: String) -> Option<Message> {
        let rest = rest.trim_start();
        let Some((recipients_str, body)) = rest.split_once(' ') else {
            return self.show_to_usage();
        };
        let body = body.trim().to_string();
        if body.is_empty() {
            return self.show_to_usage();
        }
        let recipients: Vec<String> = recipients_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if recipients.is_empty() {
            return self.show_to_usage();
        }
        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text: body,
            ts: Utc::now(),
            message_type: None,
            to: Some(recipients),
        };
        self.messages.push(msg.clone());
        self.clear_input();
        self.scroll_offset = 0;
        self.unread_count = 0;
        if self.input_history.len() >= MAX_HISTORY {
            self.input_history.remove(0);
        }
        self.input_history.push(self.input.clone());
        self.history_index = None;
        Some(msg)
    }

    fn handle_to_bare(&mut self) -> Option<Message> {
        self.show_to_usage()
    }

    fn show_to_usage(&mut self) -> Option<Message> {
        self.system_message(
            "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
        );
        self.clear_input();
        None
    }

    fn handle_view_bare(&mut self) -> Option<Message> {
        // List available channels
        let mut channel_names: Vec<&str> =
            self.active_channels.iter().map(|s| s.as_str()).collect();
        channel_names.sort();
        let list = if channel_names.is_empty() {
            "No channels yet.".to_string()
        } else {
            format!("Available channels: {}", channel_names.join(", "))
        };
        self.system_message(list);
        self.clear_input();
        None
    }

    fn handle_view_with_arg(&mut self, arg: String) -> Option<Message> {
        let arg = arg.trim();
        if arg == "all" {
            self.current_view = None;
            self.scroll_offset = 0;
            self.system_message("View set to all channels.".to_string());
            self.clear_input();
            return None;
        }
        if arg == "broadcast" {
            self.current_view = Some("broadcast".to_string());
            self.scroll_offset = 0;
            self.system_message("View set to broadcast.".to_string());
            self.clear_input();
            return None;
        }
        // Check if arg matches a known channel
        if self.active_channels.contains(arg) {
            self.current_view = Some(arg.to_string());
            self.scroll_offset = 0;
            self.system_message(format!("View set to {arg}."));
            self.clear_input();
            return None;
        }
        // No match
        self.system_message(format!(
            "no channel matches \"{arg}\". Type /view to list channels."
        ));
        self.clear_input();
        None
    }

    fn handle_pins(&mut self) -> Option<Message> {
        let list = if self.pins.is_empty() {
            "No pins yet. Use /pin <text> to pin a decision.".to_string()
        } else {
            let items: Vec<String> = self
                .pins
                .iter()
                .enumerate()
                .map(|(i, p)| format!("  {}. {}", i + 1, p))
                .collect();
            format!("Pinned:\n{}", items.join("\n"))
        };
        self.system_message(list);
        self.clear_input();
        None
    }

    fn handle_pin_with_arg(&mut self, arg: String) -> Option<Message> {
        let arg = arg.trim();
        if arg.is_empty() {
            return self.pin_usage();
        }
        if self.pins.len() >= 5 {
            self.system_message("max 5 pins — /unpin one first".to_string());
            self.clear_input();
            return None;
        }
        self.pins.push(arg.to_string());
        if let Some(dir) = &self.session_dir {
            let _ = crate::session::log::write_pins(dir, &self.pins);
        }
        self.system_message(format!("Pinned #{}: {arg}", self.pins.len()));
        self.clear_input();
        None
    }

    fn handle_pin_bare(&mut self) -> Option<Message> {
        self.pin_usage()
    }

    fn pin_usage(&mut self) -> Option<Message> {
        self.system_message(
            "Usage: /pin <text>. Example: /pin channels-as-views because filter-on-render"
                .to_string(),
        );
        self.clear_input();
        None
    }

    fn handle_unpin_with_arg(&mut self, arg: String) -> Option<Message> {
        let arg = arg.trim();
        match arg.parse::<usize>() {
            Ok(n) if n >= 1 && n <= self.pins.len() => {
                let removed = self.pins.remove(n - 1);
                if let Some(dir) = &self.session_dir {
                    let _ = crate::session::log::write_pins(dir, &self.pins);
                }
                self.system_message(format!("Unpinned #{n}: {removed}"));
            }
            _ => {
                self.system_message(format!("no pin #{arg}. Type /pins to list."));
            }
        }
        self.clear_input();
        None
    }

    fn handle_unpin_bare(&mut self) -> Option<Message> {
        self.system_message("Usage: /unpin <number>. Type /pins to see pin numbers.".to_string());
        self.clear_input();
        None
    }

    fn handle_roadmap(&mut self) -> Option<Message> {
        let roadmap_text = match fetch_roadmap() {
            Ok(text) => text,
            Err(e) => format!("Roadmap unavailable: {e}"),
        };
        self.system_message(roadmap_text);
        self.clear_input();
        None
    }

    fn handle_board(&mut self) -> Option<Message> {
        let board_text = match fetch_board() {
            Ok(text) => text,
            Err(e) => format!(
                "Board unavailable: {e}\nEnsure `gh` CLI is installed and authenticated (`gh auth login`)."
            ),
        };
        self.system_message(board_text);
        self.clear_input();
        None
    }

    fn handle_unknown_command(&mut self, text: String) -> Option<Message> {
        self.system_message(format!("Unknown command: {}. Press ? for help.", text));
        self.clear_input();
        None
    }

    fn send_chat_message(&mut self, text: String) -> Option<Message> {
        // Derive `to` from send_target if set (sidebar selection)
        let to = self.send_target.as_ref().and_then(|ch| {
            if ch == "broadcast" {
                return None; // broadcast = no `to`
            }
            // Parse channel name back to recipient list, excluding self
            let names: Vec<String> = if ch.contains('\u{2194}') {
                ch.split('\u{2194}').map(String::from).collect()
            } else if ch.contains('+') {
                ch.split('+').map(String::from).collect()
            } else {
                return None;
            };
            let recipients: Vec<String> =
                names.into_iter().filter(|n| n != &self.peer_name).collect();
            if recipients.is_empty() {
                None
            } else {
                Some(recipients)
            }
        });
        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text,
            ts: Utc::now(),
            message_type: None,
            to,
        };
        self.messages.push(msg.clone());
        self.clear_input();
        self.scroll_offset = 0;
        self.unread_count = 0;
        if self.input_history.len() >= MAX_HISTORY {
            self.input_history.remove(0);
        }
        self.input_history.push(msg.text.clone());
        self.history_index = None;
        Some(msg)
    }
}

/// Resolve the repo root: prefer `BPMUX_ROOT` env var, fall back to `git rev-parse`.
fn repo_root() -> Result<String, String> {
    if let Ok(root) = std::env::var("BPMUX_ROOT") {
        return Ok(root);
    }
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("failed to find repo root: {e}"))?;
    if !output.status.success() {
        return Err("not inside a git repository".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Read ROADMAP.md from the repo root.
fn fetch_roadmap() -> Result<String, String> {
    let root = repo_root()?;
    let path = std::path::Path::new(&root).join("ROADMAP.md");
    std::fs::read_to_string(&path).map_err(|e| format!("cannot read ROADMAP.md: {e}"))
}

/// Fetch project board from GitHub Issues via `gh` CLI.
fn fetch_board() -> Result<String, String> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args([
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "50",
        "--json",
        "number,title,state,labels",
    ]);
    // Use BPMUX_REPO env var if set, so /board works outside the git repo
    if let Ok(repo) = std::env::var("BPMUX_REPO") {
        cmd.args(["-R", &repo]);
    }
    let output = cmd.output().map_err(|e| format!("failed to run gh: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh failed: {stderr}"));
    }
    let json: Vec<serde_json::Value> =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("parse error: {e}"))?;

    struct BoardItem {
        number: u64,
        title: String,
        status: Option<String>,
    }

    let mut active: Vec<BoardItem> = Vec::new();
    let mut backlog: Vec<BoardItem> = Vec::new();
    let mut icebox: Vec<BoardItem> = Vec::new();

    for issue in &json {
        let num = issue["number"].as_u64().unwrap_or(0);
        let title = issue["title"].as_str().unwrap_or("?").to_string();
        let state = issue["state"].as_str().unwrap_or("OPEN");
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();

        if state == "CLOSED" {
            continue; // Done items not shown
        }

        // Derive status from labels for active items
        let status = if labels.contains(&"delivered") {
            Some("delivered".to_string())
        } else if labels.contains(&"finished") {
            Some("finished".to_string())
        } else if labels.contains(&"started") {
            Some("started".to_string())
        } else if labels.contains(&"rejected") {
            Some("rejected".to_string())
        } else {
            None
        };

        let item = BoardItem {
            number: num,
            title,
            status,
        };

        if labels.contains(&"icebox") {
            icebox.push(item);
        } else if labels.contains(&"backlog") {
            backlog.push(item);
        } else {
            active.push(item);
        }
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push("## Board".to_string());

    if !active.is_empty() {
        lines.push(String::new());
        lines.push(format!("### ● Active ({})", active.len()));
        for item in &active {
            let status_tag = item
                .status
                .as_ref()
                .map(|s| format!(" *{s}*"))
                .unwrap_or_default();
            lines.push(format!("- **#{}** {}{status_tag}", item.number, item.title));
        }
    }

    if !backlog.is_empty() {
        lines.push(String::new());
        lines.push(format!("### ○ Backlog ({})", backlog.len()));
        for item in backlog.iter().take(10) {
            lines.push(format!("- **#{}** {}", item.number, item.title));
        }
        if backlog.len() > 10 {
            lines.push(format!("- *+{} more*", backlog.len() - 10));
        }
    }

    if !icebox.is_empty() {
        lines.push(String::new());
        lines.push(format!("### ◌ Icebox ({})", icebox.len()));
        if let Some(first) = icebox.first() {
            lines.push(format!("- **#{}** {}", first.number, first.title));
        }
        if icebox.len() > 1 {
            lines.push(format!("- *+{} more*", icebox.len() - 1));
        }
    }

    if lines.len() <= 1 {
        Ok("No open issues found.".to_string())
    } else {
        Ok(lines.join("\n"))
    }
}
