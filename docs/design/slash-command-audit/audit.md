# Slash-command audit

*Classifying every TUI slash command by its real user and the right
interaction surface going forward.*

## The discipline

Ryan doesn't type slash commands. He uses the TUI as a visibility
surface — conversation with agents drives action, clicks on visible
affordances drive navigation. Agents use MCP tools, not TUI slash
commands.

For each existing slash command, the honest question: **who types this
today?** If the answer is "nobody," the command is dead weight.

## Classification grid

Four outcomes per command:

- **AGENT-TRIGGERED** — an agent calls an MCP tool (existing or new) to
  surface the view when the data warrants attention. Ryan doesn't type
  anything.
- **CLICKABLE** — the information is already visible somewhere in the
  TUI; add a click target to expand or navigate. Mouse-driven.
- **KEEP** — command remains as-is because it genuinely requires typed
  input that can't be reduced to a click or a conversation trigger.
- **DELETE** — no user for this command; remove it when the parent
  feature is accepted or rejected.

## The 20 commands

### `/accept <answer-code>` — **DELETE**

P2P peer-handshake acceptance. Takes a pasted code from a peer. Ryan's
interaction model is conversational, not copy-paste-code-into-TUI.
Agents have no use for it (P2P is owner-initiated).

Bundled with `/connect` in #148 (delivered, awaiting Ryan's accept).
Delete the slash command IF #148 is rejected; if accepted, keep the
protocol layer but hide the TUI surface behind a click affordance on a
P2P invitation notification.

### `/focus <roles>` — **CLICKABLE** (after implementation work)

Filter chat to specific peers. Partial infrastructure exists: the
sidebar renders peer names and clicks update `app.sidebar.selected`.
**But clicking today does NOT filter chat** — the selection state is
separate from focus filtering. Replacement story must wire sidebar
peer clicks to toggle the same `focused_roles` state `/focus` sets.

Agent usage: agents may need this via MCP for testing their own views,
but it's edge-case. Keep `/focus` as a fallback for agents; remove
Ryan's need to type it once clickable peer-name-to-focus-toggle ships.

### `/share <scope> [message]` — **DELETE (as TUI slash)**

Share last-seen message with a role or group. The agent-facing MCP tool
(#174, delivered, awaiting accept) covers the agent use case. Ryan's
use: conversational — "tell dev what I just said" — and the agent
relays via MCP. No TUI typing needed.

Keep the protocol (agents still need to propagate via MCP). Delete the
slash command once #174 is accepted.

### `/join <passphrase> <offer-code>` — **DELETE**

P2P peer handshake (initiating side). Same reasoning as `/accept` —
Ryan doesn't paste codes. Agents don't initiate P2P.

Tied to #148. Same disposition: delete when #148 resolves.

### `/to <recipients> <message>` — **AGENT-TRIGGERED**

Directed message (DM) to specific roles. Ryan's usage would be
conversational — "PM, sidebar with dev about this" — and the agent
sends via `send_message` MCP tool with a `to` filter.

For agents, the `send_message` tool already handles this via the `to`
parameter. The slash command has no user today.

Delete the slash command, keep the MCP tool. If Ryan wants visual
feedback showing which messages are DMs vs broadcast, that's a
rendering affordance in chat, not a typed command.

### `/view <channel>` — **CLICKABLE** (after implementation work)

Switch chat view to a specific channel. Sidebar shows channel names
and clicks update `app.sidebar.selected`. **But clicking today does
NOT switch the current view** — selection state is separate from
`current_view`. Replacement story must wire sidebar channel clicks to
call the same view-switching path `/view` uses.

Delete the slash command once clickable channel-name-to-view-switch
ships.

### `/search <query>` — **KEEP**

Full-text search across messages and lessons. Genuinely requires typed
input — the query IS the content. Clicking doesn't substitute for
typing what you're searching for.

Discussion: could be replaced by a "search palette" (Ctrl+K) with
typed query + clickable results. But the TYPING part is irreducible.

Keep as slash command. Consider moving trigger from `/search foo` to
clicking a "🔍 Search" affordance in the sidebar or header, which opens
an input field. That's a polish iteration, not a deletion.

### `/topic <create|list|...>` — **CLICKABLE + DELETE**

Topic channel management (#191, delivered). Has subcommands:
- `/topic create <name>` — needs typing the name
- `/topic list` — pure display, clickable
- `/topic join`, `/topic leave` — role changes, clickable per-topic

Delete subcommands that don't need typed input. Keep `/topic create`
as typed, or turn it into a clickable "+ new topic" affordance that
opens an input field.

### `/roadmap` — **AGENT-TRIGGERED**

Displays the product roadmap from a markdown file. No typed input.
Pure display — an agent could surface this when Ryan asks "what's on
the roadmap?" by calling an MCP tool that loads and shares the content.

Alternatively: it's a static view Ryan could navigate to via a clickable
header tab if the project panel grows tabs.

Delete the slash command; add an MCP tool (or use an existing one like
`view_roadmap`) that agents invoke conversationally.

### `/board` — **AGENT-TRIGGERED** (done, #219)

Board view. After #219, agents can surface the board via the poller
updating the project panel automatically. Ryan doesn't need `/board`
anymore — the data is always there when he opens the project panel.

Delete the slash command once #219 is accepted. The project panel IS
the board for Ryan; agents already use `view_board` MCP.

### `/tokens` — **AGENT-TRIGGERED**

Token usage report. Pure display. An agent surfaces it when Ryan asks
"how's our token usage?" or when velocity indicators suggest tokens
matter.

Delete the slash command; add `view_tokens` MCP tool (or repurpose an
existing one) for agent-triggered surfacing.

### `/identity [role]` — **AGENT-TRIGGERED**

Show an agent's identity traits. Pure display. An agent surfaces its
own identity when Ryan asks about it, or another agent calls
`get_identity` MCP tool (which already exists).

Delete the slash command; agent-triggered display via the MCP tool.

### `/who` — **CLICKABLE** (sidebar presence already shows this)

Lists connected peers. The sidebar's peer-presence dots already
convey this information more scannably than a text list. No typed
input.

Delete once the sidebar presentation is confirmed sufficient.

### `/quit` — **KEEP**

Exits bpmux. Typing `/quit` is already an atypical path (Ryan
Ctrl+C's the process), but it's a safety affordance. Keeping this as
a typed command is cheap and doesn't clutter.

### `/help` — **AGENT-TRIGGERED** (when useful)

Shows command help. In a model where Ryan doesn't type commands, help
becomes documentation for agents. An agent can surface relevant
command hints when Ryan asks "how do I X?" — or the help content can
shrink entirely once most commands are removed.

Defer: keep during transition, shrink or delete once the command set
is trimmed.

### `/debug` — **KEEP (dev tool)**

Toggles debug rendering. This is a developer affordance, not a user
feature. Keep as a typed command — it has one user (dev) and lives
behind a non-obvious command on purpose.

### `/status` — **DELETE**

Sets the current agent's status line text. Typed input IS the content
(like `/search`). Agents have the MCP `set_status` tool; humans use
`/status` only if they want to set their own status, which Ryan
doesn't.

Agents use the MCP tool. Nobody types this. Delete.

### `/lessons [role]` — **AGENT-TRIGGERED**

Shows saved lessons for the current agent or a specific peer. Pure
display. An agent surfaces its own lessons when Ryan asks, or via the
existing `list_lessons` / `list_peer_lessons` MCP tools.

Delete the slash command; MCP tools already cover it.

### `/session` — **KEEP (one-off utility)**

Prints the session directory path. Rare debugging use. Keep — it's
cheap, genuinely typed, and has no clickable or agent analog that
makes sense.

### `/all` — **CLICKABLE**

Toggles whether all messages or only chat-class messages display.
This IS a view filter, belongs as a clickable affordance in the
header or sidebar, not a typed command. Something like `[filter: all]`
/ `[filter: chat]` that clicks to toggle.

Replacement story: add a clickable filter toggle in the status bar or
header; delete `/all`.

## Summary table

| Command | Verdict | Notes |
|---------|---------|-------|
| /accept | DELETE | tied to #148 |
| /focus | CLICKABLE | needs wire-up: click must call focus-set path |
| /share | DELETE | MCP tool covers agent use |
| /join | DELETE | tied to #148 |
| /to | DELETE | send_message MCP covers it |
| /view | CLICKABLE | needs wire-up: click must switch view |
| /search | KEEP | typing is irreducible |
| /topic | MIXED | create typed, rest clickable |
| /roadmap | AGENT-TRIGGERED | MCP tool |
| /board | AGENT-TRIGGERED | #219 handles it |
| /tokens | AGENT-TRIGGERED | needs view_tokens MCP |
| /identity | AGENT-TRIGGERED | get_identity MCP exists |
| /who | CLICKABLE | sidebar dots already show presence |
| /quit | KEEP | safety affordance, cheap |
| /help | AGENT-TRIGGERED | shrink/delete after command trim |
| /debug | KEEP | dev tool, intentional non-obvious |
| /status | DELETE | set_status MCP tool covers it |
| /lessons | AGENT-TRIGGERED | list_lessons MCP exists |
| /session | KEEP | rare debug utility |
| /all | CLICKABLE | filter toggle in header/status bar |

**Count (of 20 commands)**: 5 DELETE, 4 CLICKABLE, 6 AGENT-TRIGGERED,
4 KEEP, 1 MIXED.

Most commands have no typing user. The audit answers Ryan's 2026-04-16
direction: the TUI is visibility, not control — surfaces should reflect
that.

## Next-step stories (for PM to shape)

1. Clickable channel names in sidebar (removes /view)
2. Clickable peer names with focus toggle in sidebar (removes /focus)
3. Delete /board TUI slash command (#219 covers the use case)
4. Agent-triggered tokens/roadmap/identity surfaces (define 2-3 small
   MCP tools if they don't exist)
5. Audit and remove /to, /share slash commands once their MCP
   equivalents are accepted (#174)
6. Resolve /accept + /join once #148 is accepted or rejected
7. /topic subcommand split: keep typed `create`, add clickable list/
   join/leave affordances

Each is a small vertical slice that removes dead weight and moves the
TUI closer to the visibility-only model Ryan uses.
