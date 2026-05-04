mod filter;
mod palette;
mod receive;
mod scroll;

use crate::session::Message;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub use super::app_state::*;

pub(super) const TRUNCATE_AFTER_LINES: usize = 10;
/// Approximate chat pane inner width (80 terminal - 2 borders).
/// `receive::compute_line_estimate` mirrors this constant — keep
/// in sync if the chat pane width changes.
const CHAT_INNER_WIDTH: usize = 78;

fn short_peer_name(peer: &str) -> &str {
    peer.split(':').next().unwrap_or(peer)
}

/// Pure search over the visible-message tail for the key (rfc3339
/// timestamp) of the most recent message that wraps past
/// `TRUNCATE_AFTER_LINES`. Pulled out of `toggle_last_truncated`
/// (#263) so the App method's mutation surface is just
/// `msg.expanded_messages` and the cross-state read of
/// `display.show_system_messages` becomes a parameter.
fn find_last_truncated_key(
    messages: &[Message],
    show_system_messages: bool,
    inner_width: usize,
) -> Option<String> {
    for msg in messages.iter().rev() {
        if !msg.is_chat() && !show_system_messages {
            continue;
        }
        let prefix_len = msg.from.split(':').next().unwrap_or(&msg.from).len() + 2;
        let full_len = prefix_len + msg.text.len();
        let wrapped = full_len.max(1).div_ceil(inner_width.max(1));
        if wrapped > TRUNCATE_AFTER_LINES {
            return Some(msg.ts.to_rfc3339());
        }
    }
    None
}

/// Owner is excluded from channel names — they're implicit in every channel.
pub fn channel_name(msg: &crate::session::Message, owner: &str) -> String {
    match &msg.to {
        None => "broadcast".to_string(),
        Some(recipients)
            if recipients
                .iter()
                .any(|r| r == crate::session::message::BROADCAST_ALL) =>
        {
            "broadcast".to_string()
        }
        Some(recipients) => {
            let sender_short = msg.from.split(':').next().unwrap_or(&msg.from);
            let mut names: Vec<&str> = recipients.iter().map(|s| s.as_str()).collect();
            if !names.contains(&sender_short) {
                names.push(sender_short);
            }
            names.retain(|n| *n != owner);
            names.sort();
            if names.is_empty() {
                sender_short.to_string()
            } else if names.len() == 1 {
                names[0].to_string()
            } else {
                names.join("+")
            }
        }
    }
}

pub struct App {
    pub identity: SessionIdentity,

    // ── Message state ──
    pub msg: MessageState,

    // ── Input editor (#260) ──
    pub editor: EditorState,

    // ── Scroll ──
    pub scroll: ScrollState,

    // ── Channel view ──
    pub channels: ChannelState,

    // ── Peers ──
    pub peers: PeerState,

    // ── Sidebar ──
    pub sidebar: SidebarState,

    // ── Search ──
    pub search: SearchState,

    // ── Project view ──
    pub project: ProjectViewState,

    // ── Tokens view (#179) ──
    pub tokens: TokensViewState,

    // ── Usage statistics (#195) ──
    /// Per-feature invocation counts: command name → (count, last_used).
    /// Incremented at dispatch time in submit_message() and palette.
    pub feature_usage: HashMap<String, (usize, chrono::DateTime<chrono::Utc>)>,

    // ── Command palette (#192) ──
    pub palette: PaletteState,

    // ── Board view (#193) ──
    pub board: BoardState,

    // ── Board poller snapshot (#219) — background-refreshed state ──
    pub board_snapshot: std::sync::Arc<std::sync::Mutex<super::board_poller::BoardSnapshot>>,

    /// Sender for off-cycle board refetches (#219 re-work). The event
    /// loop spawns the poller and stashes the tx here so view-open
    /// paths can fire `RefreshNow` when the snapshot is empty —
    /// complements the background cadence with a defensive fetch.
    /// `None` in headless mode or when the poller didn't spawn
    /// (BPMUX_REPO unset).
    pub board_poller_tx: Option<tokio::sync::mpsc::Sender<super::board_poller::PollerCommand>>,

    /// Sender for on-demand gh-CLI board fetches (#294 freeze-fix).
    /// Replaces the sync `fetch_board_stories` call that was
    /// blocking the tokio event loop under slow gh. On signal, the
    /// event loop's listener task runs the fetch via
    /// `spawn_blocking` (off-thread) and delivers the result back
    /// through `board_fetch_result_rx`, which updates both
    /// `board.stories` and `board_snapshot` on the main loop.
    /// `None` in headless mode or before the event loop starts.
    pub board_fetch_request_tx: Option<tokio::sync::mpsc::Sender<()>>,

    // ── Modal state ──
    pub input_mode: InputMode,

    // ── CI status (#151) ──
    pub(super) ci_state: crate::ci::CiState,

    // ── Display toggles ──
    pub display: DisplayState,

    // ── Layout positions (set by renderer) ──
    pub layout: LayoutState,

    // ── Lifecycle (#260) ──
    pub lifecycle: LifecycleState,
}

impl App {
    pub fn new(peer_name: String, session_id: String, messages: Vec<Message>) -> Self {
        let mut active_channels: HashSet<String> = HashSet::new();
        let peer_activity: HashMap<String, (String, Instant)> = HashMap::new();
        for msg in &messages {
            if msg.is_chat() {
                active_channels.insert(channel_name(msg, &peer_name));
            }
        }
        Self {
            identity: SessionIdentity {
                dir: None,
                name: peer_name.clone(),
                id: session_id.clone(),
            },
            msg: MessageState::new(messages),
            editor: EditorState::default(),
            scroll: ScrollState::default(),
            display: DisplayState::default(),
            peers: PeerState {
                activity: peer_activity,
                ..Default::default()
            },
            channels: ChannelState {
                active: active_channels,
                current_view: None,
                send_target: None,
                unread: HashMap::new(),
            },
            sidebar: SidebarState::default(),
            search: SearchState::default(),
            layout: LayoutState::default(),
            lifecycle: LifecycleState::default(),
            project: ProjectViewState::default(),
            tokens: TokensViewState::default(),
            // Hydrate the in-memory mirror from the persistent
            // store so the tokens overlay shows cross-session counts.
            feature_usage: crate::session::usage::load()
                .into_iter()
                .filter_map(|(k, rec)| {
                    let (_src, feature) = crate::session::usage::parse_key(&k)?;
                    let ts = crate::session::usage::last_used_datetime(&rec)?;
                    Some((feature.to_string(), (rec.count, ts)))
                })
                .collect(),
            palette: PaletteState::default(),
            board: BoardState::default(),
            board_snapshot: std::sync::Arc::new(std::sync::Mutex::new(
                super::board_poller::BoardSnapshot::default(),
            )),
            board_poller_tx: None,
            board_fetch_request_tx: None,
            input_mode: InputMode::Chat,
            ci_state: crate::ci::CiState::Unknown,
        }
    }

    /// Defensive fetch on project-view open (#219 re-work). Two
    /// parallel data sources for the panel:
    /// - `board.stories`: gh-CLI shell-out, always-available fallback,
    ///   populated by `/board` and on view-open when empty.
    /// - `board_snapshot`: octocrab poller, needs `BPMUX_REPO` +
    ///   `GH_TOKEN`. When present, drives the ambient refresh cadence.
    ///
    /// Before the re-work, only the poller was nudged on open. If
    /// the env wasn't set (Ryan's case), the poller never spawned,
    /// `request_board_refresh` was a no-op, and the panel rendered
    /// empty until `/board` was typed. Now the gh-CLI path is always
    /// primed on open — matches `/board`'s behavior without requiring
    /// the user to type it.
    pub fn request_board_refresh_if_stale(&mut self) {
        // Kick the poller when it's idle (poller handles its own
        // debounce; try_send ignores full channel).
        if let Some(tx) = &self.board_poller_tx
            && let Ok(snap) = self.board_snapshot.lock()
            && snap.last_updated.is_none()
            && snap.last_error.is_none()
        {
            let _ = tx.try_send(super::board_poller::PollerCommand::RefreshNow);
        }
        // Signal on-demand fetch via the event loop's listener task
        // when the board is empty (#294 freeze-fix). The listener
        // runs `fetch_board_stories` in `tokio::task::spawn_blocking`
        // off the event loop thread, then delivers the result back
        // through `board_fetch_result_rx` — which the main loop
        // mirrors into both `board.stories` and `board_snapshot`.
        // Before #294 this was a sync `fetch_board_stories()` call
        // here; under slow gh it froze the tokio event loop,
        // starving all other select! arms (input handling included).
        // Silent fire-and-forget: failures surface via the
        // snapshot's last_error path, the same as the poller.
        if self.board.stories.is_empty()
            && let Some(tx) = &self.board_fetch_request_tx
        {
            let _ = tx.try_send(());
        }
    }

    /// Toggle the side-by-side board panel (#318). Independent of the
    /// full-screen `InputMode::Board` path — `panel_open` coexists
    /// with chat. On open, trigger a board refetch so the panel shows
    /// current state immediately rather than the empty default.
    ///
    /// Focus cycles parallel to sidebar: closed→open+focused; focused
    /// but unfocused→focused; focused→closed.
    pub fn toggle_board_panel(&mut self) {
        if !self.board.panel_open {
            self.board.panel_open = true;
            self.board.panel_focused = true;
            self.request_board_refresh_if_stale();
        } else if !self.board.panel_focused {
            self.board.panel_focused = true;
        } else {
            self.board.panel_open = false;
            self.board.panel_focused = false;
            self.panel_close_detail();
        }
    }

    /// Move the board panel's row-list selection up (#319). Wraps at
    /// the top. No-op when the panel isn't open or the stories list
    /// is empty.
    pub fn panel_select_up(&mut self) {
        let count = self.board.stories.len();
        if count == 0 {
            return;
        }
        self.board.panel_selected = if self.board.panel_selected == 0 {
            count - 1
        } else {
            self.board.panel_selected - 1
        };
    }

    /// Move the board panel's row-list selection down (#319). Wraps
    /// at the bottom.
    pub fn panel_select_down(&mut self) {
        let count = self.board.stories.len();
        if count == 0 {
            return;
        }
        self.board.panel_selected = (self.board.panel_selected + 1) % count;
    }

    /// Open drill-in for the currently-selected panel row (#319).
    /// Resets scroll so the body starts at the top.
    pub fn panel_open_detail(&mut self) {
        if self.board.stories.is_empty() {
            return;
        }
        self.board.panel_detail = Some(self.board.panel_selected);
        self.board.panel_detail_scroll = 0;
    }

    /// Close drill-in, return to row-list view (#319). Preserves
    /// panel_selected so the list opens at the same row.
    pub fn panel_close_detail(&mut self) {
        self.board.panel_detail = None;
        self.board.panel_detail_scroll = 0;
    }

    pub fn panel_detail_scroll_up(&mut self, lines: u16) {
        self.board.panel_detail_scroll = self.board.panel_detail_scroll.saturating_sub(lines);
    }

    pub fn panel_detail_scroll_down(&mut self, lines: u16) {
        self.board.panel_detail_scroll = self.board.panel_detail_scroll.saturating_add(lines);
    }

    pub(super) fn sync_input_from_textarea(&mut self) {
        self.editor.input = self.editor.textarea.lines().join("");
        let (row, col) = self.editor.textarea.cursor();
        let line_offset: usize = self.editor.textarea.lines()[..row]
            .iter()
            .map(|l| l.len())
            .sum();
        self.editor.cursor_pos = line_offset + col;
    }

    pub(super) fn set_textarea_content(&mut self, text: &str) {
        let mut ta = tui_textarea::TextArea::from([text]);
        ta.set_hard_tab_indent(false);
        ta.set_wrap_mode(tui_textarea::WrapMode::Word);
        ta.move_cursor(tui_textarea::CursorMove::End);
        self.editor.textarea = ta;
        self.sync_input_from_textarea();
    }

    pub fn from_identity(&self) -> String {
        format!("{}:human", self.identity.name)
    }

    /// Single entry point for adding a message. Maintains class_cache
    /// and marks the filter index dirty.
    pub(super) fn push_message(&mut self, msg: Message) {
        let class = if msg.is_chat() {
            Some(crate::session::classify(&msg, &self.identity.name))
        } else {
            None
        };
        self.msg.class_cache.push(class);
        self.msg.messages.push(msg);
        self.msg.filter_dirty = true;
    }

    pub(super) fn system_message(&mut self, text: String) {
        self.push_message(Message {
            from: "bpmux".to_string(),
            text,
            ts: Utc::now(),
            ..Message::default()
        });
    }

    /// Rebuild the filtered index if dirty. Called at the start of
    /// render_chat to avoid per-frame full scans. The filter rules
    /// live in `app/filter.rs` as pure functions over a per-call
    /// `FilterContext`; this method is the index-building shell.
    pub fn rebuild_filtered_if_dirty(&mut self) {
        // Also rebuild if indices are empty but messages exist
        // (covers test setups and initial state).
        if !self.msg.filter_dirty && !self.msg.filtered_indices.is_empty() {
            return;
        }
        if self.msg.messages.is_empty() {
            return;
        }
        // Build the read-only context from disjoint sub-state fields
        // so the loop below can still take `&mut self.msg`. Inlined
        // (rather than `FilterContext::from_app(self)`) because that
        // would borrow all of `self` and block the writes.
        let ctx = filter::FilterContext {
            show_system_messages: self.display.show_system_messages,
            show_all_classes: self.display.show_all_classes,
            current_view: self.channels.current_view.as_deref(),
            identity_name: &self.identity.name,
        };
        self.msg.filter_dirty = false;
        self.msg.filtered_indices.clear();
        self.msg.cached_hidden_count = 0;
        for (idx, m) in self.msg.messages.iter().enumerate() {
            match filter::classify(&ctx, m) {
                filter::FilterVerdict::Include => self.msg.filtered_indices.push(idx),
                filter::FilterVerdict::Hidden => self.msg.cached_hidden_count += 1,
                filter::FilterVerdict::Skip => {}
            }
        }
    }

    pub fn mark_filters_dirty(&mut self) {
        self.msg.filter_dirty = true;
    }

    /// Record a slash-command invocation for usage
    /// statistics. Delegates to `session::usage::record` which
    /// loads-increments-saves the shared `~/.bpmux/usage.json` so
    /// counts accumulate across sessions (fixing #195's original
    /// overwrite-on-save loss). Also keeps an in-memory mirror for
    /// the tokens overlay.
    pub fn record_usage(&mut self, feature: &str) {
        crate::session::usage::record(crate::session::usage::Source::Slash, feature);
        let entry = self
            .feature_usage
            .entry(feature.to_string())
            .or_insert((0, chrono::Utc::now()));
        entry.0 += 1;
        entry.1 = chrono::Utc::now();
    }

    /// In-memory mirror gets persisted via `session::usage::record`
    /// on each invocation now, so this end-of-session save is a no-op
    /// kept for call-site compatibility. Left in place rather than
    /// deleted so hooks that reference it don't break.
    pub fn save_usage_to_disk(&self) {}

    /// Toggle the expanded-state of the most recent visible message
    /// that's been truncated. Mutation surface is now a single
    /// sub-state (`msg.expanded_messages`); the search itself is a
    /// pure function over the visible message list, so the read of
    /// `display.show_system_messages` becomes a parameter at the
    /// call boundary instead of an embedded field access (#263).
    pub fn toggle_last_truncated(&mut self) {
        let Some(key) = find_last_truncated_key(
            &self.msg.messages,
            self.display.show_system_messages,
            CHAT_INNER_WIDTH,
        ) else {
            return;
        };
        // For truncation (#110) the DepthLevel value doesn't matter —
        // presence in the map = "expanded past truncation threshold."
        // Reusing `DepthLevel::RoleDefault` as the sentinel so any
        // renderer checking depth sees a consistent mid-state rather
        // than a surprising Summary/Deep. Multi-depth messages don't
        // reach this path because AC #285 suppresses msg.text when
        // multi_depth is present — so there's no long-text truncation
        // to toggle.
        use crate::terminal::app_state::DepthLevel;
        use std::collections::hash_map::Entry;
        match self.msg.expanded_messages.entry(key) {
            Entry::Occupied(e) => {
                e.remove();
            }
            Entry::Vacant(e) => {
                e.insert(DepthLevel::RoleDefault);
            }
        }
    }

    pub fn update_activity(&mut self, peer: String, status: String) {
        self.peers.activity.insert(peer, (status, Instant::now()));
    }

    pub fn receive_message(&mut self, msg: Message) {
        let needs_push = receive::dispatch(self, &msg);
        if !needs_push {
            return;
        }
        // Shared tail: System and Chat-family both push + scroll-adjust.
        let was_at_bottom = self.scroll.offset == 0;
        let line_estimate = receive::compute_line_estimate(&msg, self.msg.messages.last());
        self.push_message(msg);
        if !was_at_bottom {
            self.scroll.offset = self.scroll.offset.saturating_add(line_estimate);
            self.scroll.unread_count += 1;
        }
    }
}
