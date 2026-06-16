# Agent terminal launch — starting a role as a full bpmux participant

Start (or resume) an AI role's Claude Code session **inside a bpmux terminal** so it
runs as a full team participant — sending/receiving messages, reading the board, and
firing role-actions over the `bpmux-bridge` MCP channel. This is the #1602
capability: "an agent started in a bpmux terminal is a full team participant."

## The command

Run from the role's own workspace dir (e.g. `bpmux-pm/`, `bpmux-dev/`,
`bpmux-designer/`):

```
claude --resume <session-id> \
  --plugin-dir ../bpmux/claude-plugin \
  --dangerously-load-development-channels server:bpmux-bridge \
  --add-dir ../bpmux
```

Omit `--resume <session-id>` to start a fresh session instead of resuming one.

### Flags

- `--resume <session-id>` — resume an existing Claude session by ID, keeping its full
  context. Drop it to start fresh.
- `--plugin-dir ../bpmux/claude-plugin` — load the bpmux plugin (its hooks + skills).
- `--dangerously-load-development-channels server:bpmux-bridge` — load the
  `bpmux-bridge` MCP server as a development channel (the bridge to the bpmux session).
- `--add-dir ../bpmux` — grant the session access to the shared `bpmux/` repo (the
  team's share-point).

## Why the `.mcp.json` lives in the role workspace (not the plugin)

The `bpmux-bridge` MCP server is declared at **project scope** in each role
workspace's `.mcp.json` (e.g. `bpmux-pm/.mcp.json`). It has to live there — not in
`bpmux/claude-plugin/` — because Claude Code's channel registrar only scans
enterprise / user / project / local scopes. Plugin-contributed MCP servers are not
eligible for `--dangerously-load-development-channels server:bpmux-bridge`.

If you change the bridge's startup args, update the `.mcp.json` in **every** role
workspace (`bpmux-pm/`, `bpmux-dev/`, `bpmux-designer/`).

## On connect

The session's `SessionStart` hook prompts the role to call `set_name` (e.g. `pm`,
`dev`, `designer`) so the team knows who joined. On a bridge bounce (e.g. the
operator restarts the app), re-run `set_name` on reconnect — the board-state survives
restarts (server-side authoritative), but the display-name and any in-memory
event-subscriptions / acceptance-verdicts are cleared and must be re-established
fresh.

---
Shipped by #1602. Each role workspace's `CLAUDE.md` `.mcp.json` note points here.
