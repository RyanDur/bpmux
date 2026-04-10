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
- **`GET /api/messages/search?q=...`** — search the session log
- **`GET /api/lessons`, `POST /api/lessons`** — lesson index access
- **`POST /api/click`** — simulate a mouse click at `{col, row}`
- **`POST /api/key`** — simulate a key press

The `/api/click` and `/api/key` endpoints are what make the headless instance a real desk-check platform: you can open the sidebar, navigate channels, toggle project view, and verify the frame updates correctly — all via HTTP. No second bridge, no shared state with Ryan.

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

### Example 2: verify sidebar navigation (the #145 class of bug)

```bash
BPMUX_SESSION_BASE=/tmp/bpmux-desk-check-session \
  target/debug/bpmux --mcp-port 4118 --name peer test &
sleep 2

# Open sidebar (Alt+1)
curl -s -X POST http://127.0.0.1:4118/api/key \
  -H 'Content-Type: application/json' \
  -d '{"key":"1","modifiers":"alt"}'

# Capture frame with sidebar open
curl -s http://127.0.0.1:4118/api/screen > /tmp/sidebar-open.txt

# Press right arrow to expand a section
curl -s -X POST http://127.0.0.1:4118/api/key \
  -H 'Content-Type: application/json' \
  -d '{"key":"Right"}'

curl -s http://127.0.0.1:4118/api/screen > /tmp/after-right.txt
diff /tmp/sidebar-open.txt /tmp/after-right.txt

kill %1; wait %1 2>/dev/null
```

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

## Related

- Commit `5258141` — original headless staging (#132), later broken by #141's event-loop change
- Commit `81c41a9` — headless panic fix (#132 reopen)
- `docs/diag-logging-convention.md` — companion convention about avoiding stderr writes during TUI work
- `src/terminal/event.rs` — the `next_terminal_event` helper that makes the headless path possible (`std::future::pending().await` on None stream)
