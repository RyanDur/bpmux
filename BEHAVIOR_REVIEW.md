# Behavior Review

Behaviors the team should discuss. Nothing has been removed — this is
a list for conversation.

## Behaviors that might not be needed

### Handoff system (#73)
Per-peer handoff notes (500 chars max) saved as `handoff_{name}.txt`.
Sent to the peer on WebSocket reconnect. Currently no TUI command to
write a handoff — only the bridge's `/handoff` client message creates
one. **Question: is anyone using this?** If not, it's dead weight in
the session directory and the WS connect flow.

### Pin system (#70)
Max 5 pins, managed via `/pin`, `/pins`, `/unpin`. Persisted as
`pins.json`. Sent to new WebSocket peers on connect. **Question: has
this been useful in practice?** It was designed for agents to have
persistent context, but agents now get context from CLAUDE.md and the
log.

### MCP server (src/mcp/server.rs) vs Bridge (src/bridge/mod.rs)
Two separate MCP interfaces exist:
- `BpmuxMcpServer` — runs inside the main bpmux process, exposed at
  `/mcp` endpoint
- `BridgeServer` — separate subprocess, connects back via WebSocket

Both expose the same 4 tools. **Question: do we need both?** The
bridge is what Claude Code actually uses. The `/mcp` endpoint may be
unused.

### `/board` command
Shells out to `gh issue list` to show GitHub issues categorized by
label. Requires `gh` CLI and `BPMUX_REPO` env var (or git remote).
**Question: is this still used, or do people just check GitHub
directly?**

### `/roadmap` command
Reads `ROADMAP.md` from the repo root. The 10,775-char roadmap
triggered a tui_markdown stack overflow that we had to work around
(plain text fallback for >5000 chars). **Question: is viewing the
roadmap in the TUI valuable, or is it better viewed in an editor?**

### Desktop notifications (osascript)
macOS-only. Spawns `osascript` to show a notification when a message
arrives while the TUI is unfocused. **Question: is this useful, or
is it noise?** (#47 flagged this as macOS-only tech debt.)

### `connected_peers` counter vs `peer_activity` map
The status bar shows both "N connected" (from atomic counter) and
individual peer dots (from `peer_activity` map). These track the same
concept differently. The counter includes MCP clients that never send
activity messages, so it can show "2 connected" with only 1 dot.
**Question: should we drop the counter and only show what
`peer_activity` knows about?**

### Input history (50 entries)
Up/Down cycles through previously sent messages. Max 50. Not
persisted across restarts. **Question: is 50 the right limit? Should
it persist?**

### Bell on scroll boundary
Writes `\x07` to `/dev/tty` when scrolling hits top/bottom. Hardcoded
Unix path. **Question: is the audible bell useful or annoying?**

## Behaviors that are correct but could be simpler

### Expanded message key
`expanded_messages` is a `HashSet<String>` keyed by `msg.ts.to_rfc3339()`.
This works but is fragile — if two messages have the same timestamp,
toggling one toggles both. An index-based approach would be simpler
but shifts when new messages arrive. No immediate fix needed, but
worth noting.

### Truncation estimate
Line count is estimated as `(prefix.len() + msg.text.len()) / width`.
This uses byte length, which overestimates for ASCII and underestimates
for wide Unicode. Good enough for a threshold check (>10 lines), but
not precise.

### GET /api/messages reverses then paginates
Reads all messages, reverses the entire vec, then skips/takes. For a
10,000-message log this reverses all 10,000 to return 20. Could read
from the end instead.

### Broadcast lag recovery
When the broadcast channel lags, the WS handler re-fetches all
messages from disk and replays them. Could track a cursor instead.
