# Channels-independence — designer UX-needs (research-spike, NOT-yet-story-tied)

> ⚠️ **CORRECTION 2026-06-06 — parts of this doc are REFUTED/STALE; do NOT treat as ratified guidance. Re-ground at iteration-shaping.**
> - The **"Substrate-decision RESOLVED → Push, not poll → build-native gives push-into-session"** claim is **REFUTED** (dev firsthand-grounded the Channels reference + the bridge code): mid-turn push-INTO-session is provided ONLY by Claude Code's Channels feature. bpmux's WS pushes to the BRIDGE (real push), but the **bridge→agent-SESSION hop has no native mid-turn push**. Native delivery within Claude Code = **POLL (turn-gated)** OR **own-the-agent-input-loop** (a bpmux harness via the Agent SDK — much bigger scope). This doc CONTRADICTED itself (the push-path analysis says "the agent likely POLLS"; the RESOLVED section says "push, not poll"). The REAL resolution awaits the **dev spike**, not this doc.
> - The **delivered/seen** + **coordination-overlay** UX-needs this doc shaped (became #1604 / #1605) were **OWNER-REJECTED as build-ahead** 2026-06-04 ("i never expressed a need") + reverted. Do NOT re-introduce them.
> - The **Ryan-confirmed BOUNDARIES** (bottom section) still hold; treat the rest as un-grounded shaping until re-grounded.

**Status:** research-spike shaping for the future **channels-independence** beat
(folds with reframed-Pyxis). Shaped 2026-06-02 ahead of dev (while dev builds
Carina S3), per Ryan: "do the research while dev works … stay ahead of dev
while still making them part of the process." This is the UX-NEEDS half;
**PM owns the architecture/protocol half** (A2A eval, push-vs-poll substrate,
roadmap). Dev's input lands at the iteration-plan. NOT a built design — needs
for team-planning. Story-tie when the beat is filed.

## What "channels" means (grounded — corrected from the early misread)

NOT the bpmux chat-UI. It's the **Claude Code channel** transport
(`code.claude.com/docs/en/channels-reference`, research-preview), which
bpmux-bridge IS today (the `--dangerously-load-development-channels
server:bpmux-bridge` flag). A Claude channel gives an agent session three
capabilities — these are what bpmux must deliver NATIVELY to shed the
dependency:

1. **Push-into-session** — `notifications/claude/channel` → events land in the
   agent's context as `<channel source="bpmux-bridge" …>` tags. The PUSH is the
   channel-specific magic (MCP tools are pull-only).
2. **Reply-tool** — the agent calls a tool to send back = bpmux's `send_message`.
3. **Permission-relay** — tool-approval prompts (Bash/Write/Edit) forwarded so
   they can be answered remotely; verdict echoed back by `request_id`.
+ **Identity** — the channel's configured `name` = the `source` attribute / the
  `--name` we keep in per-role `.mcp.json`.

## The independence map (what each capability becomes natively)

| Channel capability | Native bpmux replacement | UX-need (my lane) |
|--------------------|--------------------------|--------------------|
| **Reply** (agent→) | bridge-MCP `send_message` (req 2 — terminal carries the bridge) | none-new — MCP tools work pulled |
| **Permission-relay** | NATIVE in the terminal pane — the agent's own Claude Code session renders the prompt in the PTY; answer in-terminal (= #1469 "native to a real terminal", already closed-covered) | prompt renders legibly in the terminal pane; **no separate relay surface needed** |
| **Identity** | the injected `.mcp.json`'s `--name` (the req-2 identity-fork) | the identity-assignment flow (auto / prompt-on-spawn / inherit-from-cwd) |
| **Push-into-session** (→agent) | **THE HARD ONE** — MCP is pull, so without the channel notification it's poll-vs-push (PM+dev substrate-Q) | see below |

## The load-bearing UX-need: the push-path

When Ryan reaches an agent, there are **two distinct paths** with different feel:

- **Terminal-stdin** (Ryan types in the agent's terminal pane) — INSTANT, native
  PTY. This is the primary "talk to an agent in its terminal" Ryan asked for.
  Already delivered by the native terminal (S1 full-duplex). No push needed.
- **Chat-UI message** (Ryan/another agent sends a bpmux message TO the agent) —
  needs to REACH the agent's session. Today the Claude-channel PUSHES it. Native,
  without the channel, the agent likely POLLS a bridge read-tool on its
  turn-boundary → a **latency/responsiveness** UX gap (the agent doesn't react
  instantly to a chat-message the way it does to terminal-stdin).

**UX-needs:**
1. A chat-message addressed to an agent must reach it + the agent's response must
   surface back in the chat-UI — keeping Ryan's boundary: the chat goes through
   the conversation-layer (bridge), NOT injected raw into the terminal-session
   stream (session-local ≠ conversation-layer).
2. If poll-based: the responsiveness gap needs a legible surface (e.g. a
   pending/delivered/seen state on the message, or a "nudge" affordance), so a
   not-yet-polled message doesn't read as a dropped/ignored one. (Sister to the
   #1599 invisible-state→legible-state class.)
3. Terminal-stdin stays the instant path; the chat-message path is the
   coordinated/async one. The UX should make the two paths' feel honest (don't
   promise instant where it's polled).

## Observability (persists — bpmux-native, NOT at-risk)

The chat-UI, team-conversations, and Ryan-watching-the-team all PERSIST (they're
bpmux's own surface, not Claude-channel-dependent). The future-need if agents
coordinate sideways (A2A candidate, PM's eval): the chat-UI must SURFACE
agent↔agent coordination legibly, not just human↔agent — so Ryan keeps the
bird's-eye even when agents talk to each other directly. (This is the
team-observability half I flagged; it's a rendering-need on the persisting UI,
not a new transport.)

## Substrate-decision RESOLVED (2026-06-02, spike #1603 — canonical home: the story + ROADMAP L193)

The sideways-coordination + push-vs-poll substrate Qs below are **resolved**:
**build-native internal** (bpmux's own WebSocket+MCP), A2A reserved as a future
*external* interop adapter only. Grounds (dev, line-level vs the FULL A2A spec):
A2A's two push modes both make the receiver own infra (SSE hold-open / webhook
server) = heavy for N local agents; A2A is point-to-point (no native broadcast →
bpmux builds fan-out regardless); A2A-Rust is 0.x. Converges with ROADMAP L193
("A2A is a delegation protocol, not a participation protocol… bpmux's own
WebSocket+MCP stack handles collaboration") — ratifies existing canon, not a pivot.

**UX-implications of the resolution (carry into N+1 spec-shaping):**
- **Push, not poll** — build-native gives push-into-session. So UX-need #2 below
  reframes from "*required if poll-based*" to: delivered/seen state still warranted
  because **pushed ≠ processed-at-turn-boundary** (the agent may not have reached
  its turn to consume a pushed message). The responsiveness surface persists; its
  justification shifts from latency-hedge to processed-state-legibility.
- **Observability render-target** — the surface renders **bpmux-native coordination
  events** (the chat-UI's existing vocabulary: send_message fan-out, delivered/seen),
  **NOT** A2A's task-lifecycle (submitted/working/completed). Simpler target; no
  foreign state-vocabulary to introduce. (Supersedes the A2A-lifecycle framing that
  was on the old "## Open questions" A2A bullet.)

## Open questions for team-planning (dev's input)

- **Push-vs-poll** — RESOLVED above (push, build-native). Retained for trace.
- **Identity assignment** (#2 fork): auto-gen / prompt-on-spawn / inherit-from-
  cwd-workspace's `.mcp.json` / explicit. Dev grounded the mechanism (inject
  project-scope `.mcp.json` at spawn-cwd → identity = its `--name`). STILL OPEN.
- **A2A** — RESOLVED above (build-native; A2A external-interop-only). The
  observability-surface renders bpmux-native events, not A2A task-lifecycle.

## #1320-slice UX-survivability mirror (pairs with PM's user-job audit)

For each rooms-as-homes slice — does its SURFACE reframe under native-terminal,
or die? (PM audits the user-job survivability; this is the UX-surface mirror.)
Finding: the slices REFRAME — none dies at the *user-job* level; what dies is
the agent-ROOM-specific chrome (asymmetric-rendering, permission-in-room).

| #1320 slice | UX-surface verdict | reframes to |
|-------------|--------------------|-------------|
| owner-view **multi-pane** | DEFERRED-REFRAME | terminal SPLITS (iTerm-style, see N terminals at once). Deferred-want (Ryan: one-at-a-time fine for now); the team-observability bird's-eye is the sister need. |
| **MCP-provisioning** | REFRAME (= #2) | terminal injects project-scope `.mcp.json`; surface is mostly transparent + the identity-flow. |
| **agent-diversity** | REFRAME | run ANY agent-CLI in a terminal — no agent-picker surface; you just run the command. |
| **portability** | REFRAME / DEFER | native shell powers (ssh in the terminal) cover near-term; a dedicated remote-terminal surface is a deferred-want. |
| **onboarding** | REFRAME | spawn-agent-in-a-terminal + identity-assignment (#2 fork) IS onboarding; the flow is the spawn-flow, not a separate wizard. |
| **cross-room-comm** | REFRAME | "terminals-not-channels" → the agent↔agent substrate (A2A, PM) + the team-observability SURFACE (chat-UI renders the coordination — mine). |
| asymmetric-rendering / permission-in-room | DIE | agent-room chrome; gone with the agent-rooms model (permission is now native-in-terminal-pane per #1469). |

**Survivor UX-surfaces that become reframed-Pyxis stories:** terminal-splits
(multi-pane, deferred), the identity-flow (#2), the team-observability
surface, the onboarding spawn-flow. The push-surface (above) is the
channels-independence half.

## Ryan's boundaries this must hold (confirmed this session)

- Terminal-session I/O stays LOCAL — never auto-spills to broadcast.
- Talk-to-the-team-as-now PERSISTS (chat-UI is not retired).
- Sharing terminal content to the team is EXPLICIT (copy [S7] or the agent
  chooses to `send_message`), never automatic.
