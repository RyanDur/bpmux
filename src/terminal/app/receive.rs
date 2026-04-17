//! Per-MessageType reducers for `App::receive_message` (#261).
//!
//! Each reducer takes narrow `&mut` refs to the sub-states it
//! actually touches. Compared to the original ~100-line god method,
//! the reducer signatures document coupling at the function level
//! instead of hiding it inside `&mut self` field accesses.
//!
//! `App::receive_message` is now a thin dispatcher that branches on
//! `MessageType`, calls the right reducer, then runs the shared
//! "compute line estimate, push, adjust scroll" tail (System and
//! Chat-family messages all share that tail; Activity short-circuits
//! before it).
//!
//! Coverage gates pinned in `terminal::app_tests::receive_message_*`
//! before this split landed — see the gate notes in #261.
//!
//! Approximate chat pane inner width used for wrap math; matches
//! the constant in `app/mod.rs`.
const CHAT_INNER_WIDTH: usize = 78;

const DM_FLASH_DURATION: std::time::Duration = std::time::Duration::from_millis(750);

use std::time::Instant;

use crate::session::{Message, MessageType};

use super::{
    ChannelState, MessageState, PeerState, SessionIdentity, channel_name, short_peer_name,
};

/// Activity message: bump the sender's presence entry. No log push.
pub(super) fn handle_activity(peers: &mut PeerState, sender: String, text: String) {
    peers.activity.insert(sender, (text, Instant::now()));
}

/// System message: handle peer join/left transitions (which mutate
/// presence + the channel-list snapshot).
pub(super) fn handle_system(
    channels: &mut ChannelState,
    peers: &mut PeerState,
    msg_state: &MessageState,
    identity: &SessionIdentity,
    text: &str,
) {
    if let Some(peer) = text.strip_suffix(" joined") {
        let short = short_peer_name(peer).to_string();
        peers
            .activity
            .insert(short.clone(), ("available".into(), Instant::now()));
        channels.active.insert(short);
        rebuild_channels(channels, msg_state, peers, &identity.name);
    } else if let Some(peer) = text.strip_suffix(" left") {
        peers.activity.remove(short_peer_name(peer));
        rebuild_channels(channels, msg_state, peers, &identity.name);
    }
}

/// Chat-family message (Chat / Insight / Nudge): channel registration,
/// unread bookkeeping, /status side-effect, and DM-flash tracking.
/// Does NOT push the message — the caller does that as part of the
/// shared tail so all non-Activity branches share scroll-preservation
/// logic.
pub(super) fn handle_chat(
    channels: &mut ChannelState,
    peers: &mut PeerState,
    identity: &SessionIdentity,
    msg: &Message,
    sender: String,
) {
    let ch = channel_name(msg, &identity.name);
    channels.active.insert(ch.clone());
    if channels.current_view.as_ref().is_none_or(|v| v != &ch) {
        *channels.unread.entry(ch).or_insert(0) += 1;
    }
    if let Some(rest) = msg.text.strip_prefix("/status ") {
        peers
            .activity
            .insert(sender.clone(), (rest.trim().to_string(), Instant::now()));
    }
    if let Some(recipients) = &msg.to {
        let directed_at_me = recipients.iter().any(|r| r == &identity.name)
            && !recipients
                .iter()
                .all(|r| r == crate::session::message::BROADCAST_ALL);
        if directed_at_me && !peers.dm_unread_from.contains(&sender) {
            peers.dm_unread_from.push(sender);
            peers.dm_flash_until = Some(Instant::now() + DM_FLASH_DURATION);
        }
    }
}

/// Estimate how many rendered lines this message will occupy so
/// `receive_message` can compensate `scroll.offset` and keep the
/// viewport stable when the user is scrolled up. The estimate is
/// approximate (within ±1 line) — exact wrap math would need the
/// renderer's font metrics, which aren't available here.
///
/// Pure function over (msg, last logged message). Extracted from
/// receive_message so it's testable without an App.
pub(super) fn compute_line_estimate(msg: &Message, last: Option<&Message>) -> usize {
    let sender = msg.from.split(':').next().unwrap_or(&msg.from);
    let prefix_len = sender.len() + 15;
    let content_width = CHAT_INNER_WIDTH.saturating_sub(prefix_len).max(1);
    let wrapped = msg.text.len().max(1).div_ceil(content_width).max(1);
    let separator = last
        .map(|m| {
            let prev = m.from.split(':').next().unwrap_or(&m.from);
            if prev != sender { 1 } else { 0 }
        })
        .unwrap_or(0);
    wrapped + separator
}

/// Recompute `channels.active` from the message log + presence map.
/// Used by the System reducer when a peer joins or leaves; was
/// previously a private method on App. Kept narrow on purpose —
/// only the four touchpoints it actually needs.
fn rebuild_channels(
    channels: &mut ChannelState,
    msg_state: &MessageState,
    peers: &PeerState,
    identity_name: &str,
) {
    let connected: std::collections::HashSet<&str> =
        peers.activity.keys().map(|s| s.as_str()).collect();
    channels.active = msg_state
        .messages
        .iter()
        .filter(|m| m.is_chat())
        .map(|m| channel_name(m, identity_name))
        .filter(|ch| {
            if ch == "broadcast" {
                return true;
            }
            let has_connected_peer = ch.split('+').any(|name| connected.contains(name));
            let has_unread = channels.unread.get(ch).is_some_and(|&n| n > 0);
            has_connected_peer || has_unread
        })
        .collect();
}

/// Dispatches the message to the matching reducer and notes whether
/// the caller still needs to push + scroll-adjust. Activity short-
/// circuits (returns false); System and Chat-family return true so
/// the shared tail runs.
pub(super) fn dispatch(app: &mut super::App, msg: &Message) -> bool {
    let sender = short_peer_name(&msg.from).to_string();
    match msg.message_type {
        MessageType::Activity => {
            handle_activity(&mut app.peers, sender, msg.text.clone());
            false
        }
        MessageType::System => {
            handle_system(
                &mut app.channels,
                &mut app.peers,
                &app.msg,
                &app.identity,
                &msg.text,
            );
            true
        }
        MessageType::Chat | MessageType::Insight | MessageType::Nudge => {
            handle_chat(
                &mut app.channels,
                &mut app.peers,
                &app.identity,
                msg,
                sender,
            );
            true
        }
    }
}
