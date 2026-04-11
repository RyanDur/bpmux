# Desk-check recipe: live TUI verification via headless staging

*Designer — 2026-04-10. Proves the workflow unblocked by #132 (headless mode no longer panics).*

## Why this recipe exists

Before #132 landed, the only way to desk-check a TUI change was to read the code or wait for Ryan to exercise it in the live bpmux. Both of those failed this morning: the `#144` stderr-bleed regression shipped with sign-off from three code reviews, but nobody ran the TUI. The result was an unreadable frame the first time Ryan opened the app.

This recipe gives any agent an independent way to render, inspect, and interact with a bpmux TUI — without touching Ryan's screen and without a second bridge connection. Use it for sign-off on anything that could affect the rendered frame or terminal event handling.

## The short version

```bash
# In the repo root after `cargo build`:
BPMUX_SESSION_BASE=/tmp/bpmux-desk-check-session \
  target/debug/bpmux \
    --mcp-port 4118 \
    --name desk-check-peer \
    test &
sleep 2

# Inspect the rendered frame:
curl -s http://127.0.0.1:4118/api/screen

# Post a message and see it render:
curl -s -X POST http://127.0.0.1:4118/api/messages \
  -H 'Content-Type: application/json' \
  -d '{"from":"desk-check-peer","text":"hello"}'
curl -s http://127.0.0.1:4118/api/screen

# Clean up:
kill %1
wait %1 2>/dev/null
```

## The three environment adjustments

These three flags / env vars are mandatory. Without them, the test instance collides with Ryan's live bpmux:

| Knob | Value | Why |
|------|-------|-----|
| `--mcp-port` | `4118` (or any free port ≠ 4117) | Ryan's live bpmux binds 4117. Binding the same port from the test instance fails immediately |
| `BPMUX_SESSION_BASE` | `/tmp/...` (or any dir outside `~/.bpmux/sessions`) | The default session base is shared. Two instances writing to the same session dir corrupt each other's state |
| `--name` | anything descriptive, e.g. `desk-check-peer` | Makes log entries and message attribution legible; defaults to `dev` which collides |

## What you can actually do

The headless instance exposes the same REST API as Ryan's live bpmux — it's not a special "test" API, it's just `bpmux` with a `TestBackend` instead of a `CrosstermBackend`. Endpoints:

- **`GET /api/screen`** — returns the 80×24 rendered frame as plain text. Use this to assert visual correctness
- **`POST /api/messages`** — inject a message into the session. Useful for setting up state before inspecting the frame
- **`GET /api/messages?limit=N`** — read recent messages
- **`GET /api/search?query=...`** — search the session log
- **`GET /api/lessons`, `POST /api/lessons`** — lesson index access
- **`GET /api/peer-lessons`** — read another role's lesson index (cross-role visibility for review contexts)
- **`POST /api/simulate/click`** — simulate a mouse click at `{col, row}`
- **`POST /api/simulate/key`** — simulate a key press (payload: `{"key": "<name>"}`, see modifier note below)
- **`POST /api/classifier-misfire`** — report a classifier nudge that fired on a non-learning-moment, for tuning data

The `/api/simulate/click` and `/api/simulate/key` endpoints are what make the headless instance a real desk-check platform: you can navigate channels, trigger handlers, and verify the frame updates correctly — all via HTTP. No second bridge, no shared state with Ryan.

**Known limitation — key modifiers**: the current `POST /api/simulate/key` handler accepts only `{"key": "<name>"}` and ignores any `modifiers` field. This means key combinations like `Alt+1` (toggle sidebar), `Ctrl+C`, or `Shift+Tab` cannot be simulated via the REST endpoint today. Plain keys (`enter`, `esc`, `tab`, `backspace`, arrows, letters, digits) work. See #170 for the follow-up that will add modifier support.

## Workflow examples

### Example 1: verify a TUI change renders cleanly (the #144 class of bug)

```bash
# Start the instance
BPMUX_SESSION_BASE=/tmp/bpmux-desk-check-session \
  target/debug/bpmux --mcp-port 4118 --name peer test &
sleep 2

# Make the TUI do some work — post a bunch of messages to exercise storage
for i in 1 2 3 4 5; do
  curl -s -X POST http://127.0.0.1:4118/api/messages \
    -H 'Content-Type: application/json' \
    -d "{\"from\":\"peer\",\"text\":\"test $i\"}"
done

# Capture the frame and verify it's clean
curl -s http://127.0.0.1:4118/api/screen | tee /tmp/desk-check-frame.txt

# Look for stderr bleed — if any `eprintln!` fires during the posts, it would
# show up as garbled text mixed into the frame. If the frame is clean, no bleed.
grep -E '[A-Z][a-z]+: ' /tmp/desk-check-frame.txt | grep -v '\[broadcast\]' \
  && echo "POSSIBLE STDERR BLEED — investigate" \
  || echo "frame clean"

kill %1; wait %1 2>/dev/null
```

### Example 2: verify `/help` slash command renders (the #164 class of bug)

```bash
BPMUX_SESSION_BASE=/tmp/bpmux-desk-check-session \
  target/debug/bpmux --mcp-port 4118 --name peer test &
sleep 2

# Type "/help" one character at a time (no modifiers needed)
for ch in "/" "h" "e" "l" "p"; do
  curl -s -X POST http://127.0.0.1:4118/api/simulate/key \
    -H 'Content-Type: application/json' \
    -d "{\"key\":\"$ch\"}"
done

# Press enter to submit
curl -s -X POST http://127.0.0.1:4118/api/simulate/key \
  -H 'Content-Type: application/json' \
  -d '{"key":"enter"}'

# Capture frame with /help output rendered
curl -s http://127.0.0.1:4118/api/screen > /tmp/help-output.txt

# Verify expected commands appear. Note: the 80-col TUI wraps long lines,
# so longer command names (/share, /lessons) can be split across row
# boundaries as e.g. "/s" + "hare". Either grep for unwrapped short names
# OR strip the line-wrap artifacts (newlines + column-border chars) before
# matching. The strip-and-match form catches any command regardless of
# wrapping:
grep -q "Commands:" /tmp/help-output.txt && echo "help header rendered"
grep -q "/to" /tmp/help-output.txt && echo "/to listed"
tr -d '│\n ' < /tmp/help-output.txt | grep -q "/share" && echo "/share listed"

kill %1; wait %1 2>/dev/null
```

**Why this example instead of sidebar navigation**: the `/help` slash command path exercises plain-key simulation (characters + enter) end-to-end without needing any modifier keys. The original sidebar-navigation example — which used `Alt+1` to open the sidebar — cannot currently be expressed via the REST endpoint because `/api/simulate/key` ignores the `modifiers` field. Once #170 lands that modifier support, this section should be expanded with a sidebar-navigation workflow using `Alt+1` + arrow keys for a true #145-class test.

## When to use this recipe

- **Before accepting any story that touches the `terminal::` module** — render verification
- **Before accepting any story that adds logging, instrumentation, or I/O that could hit the tty** — stderr bleed verification
- **Before accepting any story that changes keyboard or mouse handling** — interaction verification
- **When reproducing a TUI bug Ryan reports** — isolates the bug from your live session

## When NOT to use this recipe

- For P2P testing (#76 class of story). Two `bpmux test` instances don't handshake with each other automatically — they'd need the full `/session` / `/join` / `/accept` flow, which requires coordinated offers and answers. That's a separate workflow and out of scope for this doc.
- For testing Claude Code hooks or permissions. Those live in the Claude Code harness, not the bpmux server.
- For testing external services (GitHub, MCP servers outside the bpmux repo).

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `Address already in use` | Another instance is on the same port | Change `--mcp-port` or `kill %1` previous instance |
| `No such file or directory` after startup | Session base not writable | Use `/tmp/...` or another writable dir |
| Frame is the same across captures | Event loop is idle, nothing changed | Post a message or simulate a key press to trigger a render |
| `curl` hangs | Server not fully booted | Increase the `sleep 2` before the first request |
| `bpmux test` exits immediately | #132 regression | Rebuild with `cargo build` and verify binary is `adbd76e` or later |
| `POST /api/click` or `POST /api/key` returns 404 | Old endpoint path from an earlier draft of this recipe | Use `/api/simulate/click` and `/api/simulate/key` — the `simulate/` prefix is required |
| `GET /api/messages/search?q=...` returns 404 | Wrong search path + wrong param name | Use `GET /api/search?query=...` — different path and the param is `query`, not `q` |
| Key-combination (`Alt+1`, `Ctrl+C`) has no visible effect | The `modifiers` field on `/api/simulate/key` is currently ignored — only plain keys are dispatched | Use plain-key workflows until #170 lands modifier support. See the "Known limitation — key modifiers" note in the endpoints section |

## Related

- Commit `5258141` — original headless staging (#132), later broken by #141's event-loop change
- Commit `81c41a9` — headless panic fix (#132 reopen)
- `docs/diag-logging-convention.md` — companion convention about avoiding stderr writes during TUI work
- `src/terminal/event.rs` — the `next_terminal_event` helper that makes the headless path possible (`std::future::pending().await` on None stream)
