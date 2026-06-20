# Terminal message-injection model

How bpmux delivers a chat/broadcast message *into* an agent's terminal "off
Claude's Channels" (the #1656 keystone), and why the in-progress-input
collision happens. Written at the Crux retro to close a three-seat
knowledge-gap: dev, PM, and designer all initially mis-modelled where the
user's partial input lives. It lives in the **kernel line-discipline**, not the
renderer.

## Where the in-progress input lives

When a user types into an agent's terminal, the partial (un-submitted) line is
held by the **PTY kernel line-discipline in cooked mode** — not in xterm.js, not
in any bpmux buffer. The renderer (`@xterm/xterm`,
`applications/electron/src/renderer/src/chat/TerminalView.tsx`) streams every
keystroke immediately: `term.onData` → WebSocket → `handle_pty_input`
(`applications/bpmux/src/transport/ws/client_handlers.rs`) → the PTY master.
xterm.js keeps the *display* state; the kernel keeps the *input* state. Once a
carriage-return reaches the kernel, the line is committed to the agent — there
is nothing client-side to restore.

## The shared writer

`LivePty` (`applications/bpmux/src/room/spawn/pty.rs`) holds one
`Mutex<Box<dyn Write>>` — the PTY master writer — and BOTH input paths write to
it:

- `write_input` (keystrokes): user keystroke bytes, written raw.
- `submit_input` (injection): a delivered message, wrapped by `frame_for_paste`,
  then a `SUBMIT` byte that is a single `\r`.

They serialise per-`write_all`, but they share the same kernel line-discipline
buffer across calls. The `\r` does not submit "the injected message" — it
submits whatever has accumulated in that buffer.

## The collision (the two #1656 defects)

1. **Typing-swept** (the serious one): the user is mid-typing, the loop injects
   a framed line + `\r`, and the `\r` commits the user's unfinished text
   *together with* the message as one line. The user's keystrokes still show in
   xterm (display state), but the agent receives a merged/corrupted line.
2. **Stuck / parked** (intermittent): a bare `\r` to a busy or paste-off agent
   parks in the buffer awaiting a manual Enter. Bracketed-paste framing
   (`frame_for_paste`, `applications/bpmux/src/room/spawn/paste_mode.rs`)
   mitigates this for paste-aware agents by submitting as a deliberate
   paste-entry; it is a heuristic, not a decouple. The `pty_tests.rs` "PARK"
   test documents the owner's 2026-06-13 intermittent reject.

## Implication for the rework

The fix is to decouple injection from the keystroke buffer. Two shapes:

- **Relocate input-ownership client-side**: the renderer owns the in-progress
  line (client-side line-editing, submit on Enter), so an injection can
  capture/restore the partial line, or defer while the user is typing, instead
  of racing the kernel buffer. This is the web-environment-native direction.
- **A separate injection channel** that bypasses the shared line-discipline.

What does NOT fix it: calling xterm.js `term.input()` / `term.paste()` alone —
those route the same `onData → WS → PTY-stdin` path and still race the keystroke
stream. The fix is relocating *where the input-line lives*, which is more than
one API call.
