mod filter;
mod palette;
mod receive;
mod scroll;

use crate::session::{Message, MessageType};
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

    // ── Focus mode part A (#181) ──
    /// When `Some(roles)`, the chat view filter narrows to messages
    /// from the listed roles. Transient per-session (resets on
    /// restart) — per story decision. `None` = no filter.
    /// See `commands_focus::in_focus` for the override rules
    /// (own messages, system messages, addressee=my_role bypass).
    pub focused_roles: Option<Vec<String>>,

    // ── Command palette (#192) ──
    pub palette: PaletteState,

    // ── Threads (#268) ──
    pub threads: ThreadState,

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

    // ── Modal state ──
    pub input_mode: InputMode,

    // ── CI status (#151) ──
    pub(super) ci_state: super::ci::CiState,

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
            focused_roles: None,
            palette: PaletteState::default(),
            threads: ThreadState::default(),
            board: BoardState::default(),
            board_snapshot: std::sync::Arc::new(std::sync::Mutex::new(
                super::board_poller::BoardSnapshot::default(),
            )),
            board_poller_tx: None,
            input_mode: InputMode::Chat,
            ci_state: super::ci::CiState::Unknown,
        }
    }

    /// Defensive fetch on project-view open (#219). If the snapshot
    /// has never returned data, signal the poller to fetch now
    /// instead of waiting for the next cadence tick — "initializing..."
    /// stops meaning "poller is silent for 60s" and starts meaning
    /// "actual fetch in flight." No-op when a fetch has already
    /// succeeded or a fetch is already in flight.
    pub fn request_board_refresh_if_stale(&self) {
        let Some(tx) = &self.board_poller_tx else {
            return;
        };
        let needs_refresh = match self.board_snapshot.lock() {
            Ok(snap) => snap.last_updated.is_none() && snap.last_error.is_none(),
            Err(_) => return,
        };
        if needs_refresh {
            let _ = tx.try_send(super::board_poller::PollerCommand::RefreshNow);
        }
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
            focused_roles: self.focused_roles.as_deref(),
            current_thread: self.threads.current.as_deref(),
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
        if self.msg.expanded_messages.contains(&key) {
            self.msg.expanded_messages.remove(&key);
        } else {
            self.msg.expanded_messages.insert(key);
        }
    }

    pub fn update_activity(&mut self, peer: String, status: String) {
        self.peers.activity.insert(peer, (status, Instant::now()));
    }

    /// Leave the current thread (#268) — clear the thread view and
    /// reset the peripheral counter that tracked broadcast activity
    /// while focused inside. No-op when already in broadcast view.
    pub fn leave_thread(&mut self) {
        if self.threads.current.take().is_some() {
            self.threads.broadcast_unread_while_threaded = 0;
            self.mark_filters_dirty();
        }
    }

    pub fn receive_message(&mut self, msg: Message) {
        // Peripheral counter (#268): while the viewer is inside a
        // thread, bump the broadcast-unread counter whenever an
        // incoming message ISN'T part of the current thread. Keeps
        // cross-thread activity visible without breaking scope.
        if let Some(current) = self.threads.current.as_deref()
            && msg.message_type == MessageType::Chat
            && msg.thread_id.as_deref() != Some(current)
            && msg.from != self.from_identity()
        {
            self.threads.broadcast_unread_while_threaded += 1;
        }
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
