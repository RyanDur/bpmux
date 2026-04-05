# Claude Code channels — dependency risk

bpmux's real-time push into Claude Code depends on **channels**
(`--dangerously-load-development-channels`), which is experimental /
research-preview only.

## Risk

High. No stability guarantee, no deprecation period, org admins can
disable silently, protocol may change without notice. Precedent: Claude
4.6 removed prefilling with a hard 400 error, no grace period.

## Why we depend on it

No alternative exists today for push notifications from an MCP server
into a Claude Code agent's context. Standard MCP is request-response
only; channels is the only push mechanism currently available.

## Degradation path

We're designed so we're never fully down:

1. **Channels working** → real-time push into Claude Code context.
2. **Channels broken, hooks working** → per-turn sync via a Stop hook
   that polls bpmux's REST API.
3. **Both broken** → bridge still connected via WebSocket. bpmux can
   poll its own clients or buffer messages. The human sees everything
   in the TUI regardless.

The transport layer (WebSocket, REST API) is ours. We control it. The
channels dependency is only the last mile into Claude Code's
conversation context.

## What this means for design

- Don't assume channels will always work. Design features that degrade
  gracefully.
- The hook-based path must remain the reliable baseline, not an
  afterthought.
- bpmux-side polling is a third fallback if hooks also fail.
- Monitor channel delivery — if inbound ≠ outbound message counts,
  channels may have silently failed.
- When giving bpmux to other users, the hook path is safer than
  channels for initial experience.
- WebRTC is the long-term path to P2P transport that doesn't depend on
  Claude Code's experimental APIs.
