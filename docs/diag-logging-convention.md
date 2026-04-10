# Diag Logging Convention

*Designer — 2026-04-10. Based on two incidents in 48 hours where stderr writes from background code corrupted the live TUI frame.*

## The rule

**Any code that runs concurrently with the TUI must not write to `stdout` or `stderr`.** If you need to log, route through an environment-gated file sink using the diag pattern described below.

This applies to:
- Background tasks spawned via `tokio::spawn`
- Decorators / instrumentation layers wrapping long-lived objects
- Error paths in the session, transport, and broadcast layers
- Anything reachable from code that runs after `terminal::init()` sets up the alt-screen

It does not apply to:
- Code that runs **before** the TUI initializes (startup banners, CLI argument errors)
- Code that runs **after** the TUI tears down (reload messages, shutdown errors)
- Code in a **separate process** that doesn't own the same tty (e.g. `src/bridge/mod.rs` — the bridge runs in its own process spawned by Claude Code and does not touch bpmux's tty)
- `main.rs` startup error handling before the terminal is taken over

When in doubt, assume the TUI is live and use the diag pattern.

## Why this rule exists

Ratatui takes over `stdout` via the terminal's alternate screen buffer. While the alt-screen is active, every character that lands on the tty — from any source — is interpreted as part of the rendered frame. Normal prints land in the middle of cells, ANSI escape sequences collide with ratatui's cursor positioning, and the result is a scrambled, unreadable frame.

`stderr` does **not** go through ratatui's alt-screen. It flows directly to the same tty that the alt-screen is painting on. That means `eprintln!` from a background tokio task is indistinguishable from a render bug: you get garbled overlapping text until the next full frame redraws.

### Incident 1: InstrumentedStorage stderr bleed (2026-04-10 morning, fix in `928cbc4`)

The `InstrumentedStorage` decorator was added to capture storage-operation timings for a storage-backend decision (#144). It used `eprintln!` to log each operation's duration. Every `append_message` call — firing several times a second during normal chat — wrote a timing line to stderr, which landed in the middle of the TUI frame. The user's first experience in the morning was a completely unreadable chat pane with `Storage: append_message 0.2ms` lines bleeding through every broadcast.

Root cause: the unit tests for `InstrumentedStorage` passed cleanly (stderr output isn't rendered during tests), and the PR was signed off on code review. No one ran a live TUI against the decorator. The bug was only visible in a rendered frame.

Lesson: **code review is insufficient for anything that could write to stdout/stderr. Desk-check against a live TUI frame.**

### Incident 2: Background-task eprintln cleanup (2026-04-10 morning, fix in `adbd76e`)

After #144 was fixed, a grep found five more `eprintln!` sites that would corrupt the TUI under error conditions:

- `src/transport/ws.rs` — session-error branch, lagged-broadcast handler, read-error branch
- `src/lib.rs` — `server::run_server` spawned-task error
- `src/terminal/event.rs` — broadcast-recv lagged recovery

Each was a rare path (fires only on error) but would corrupt the frame when it fired. All were replaced with calls to `session::diag::log_diag(...)`.

## The diag pattern

### Module

`src/session/diag.rs` defines a small helper:

```rust
pub fn log_diag(line: &str) {
    let Ok(path) = std::env::var("BPMUX_LOG") else {
        return;
    };
    if path.is_empty() {
        return;
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    let ts = chrono::Utc::now().to_rfc3339();
    let _ = writeln!(file, "{ts} {line}");
}
```

### Shape of a diag call

Instead of:

```rust
eprintln!("ws: session error: {e}");
```

Write:

```rust
crate::session::diag::log_diag(&format!("ws: session error: {e}"));
```

### Invariants

1. **Opt-in by default.** When `BPMUX_LOG` is unset, logging is a no-op. No side effects, no file creation, no surprise.
2. **Silent on error.** If the file can't be opened or written, the call silently returns. The caller has no way to report the error that wouldn't itself risk writing to the tty.
3. **Timestamped.** Every line is prefixed with an RFC 3339 UTC timestamp so the log is chronologically analyzable.
4. **Append-only.** The file is opened with `create: true, append: true`. Concurrent writers from different tokio tasks won't corrupt each other's lines for small writes.

## Other instances of this pattern

The diag pattern is the same shape as two other env-gated file sinks in the codebase:

| Env var              | Sink                                                 | Purpose                                           |
|----------------------|------------------------------------------------------|---------------------------------------------------|
| `BPMUX_LOG`          | `session::diag::log_diag`                             | Background-task error reporting                   |
| `BPMUX_STORAGE_LOG`  | `session::instrumented_storage::InstrumentedStorage` | Storage operation timings (from #144)             |
| `BPMUX_HOOK_LOG`     | `.claude/hooks/auto-approve-bash.sh` logger         | PreToolUse Bash hook dry-run / enforcement audit  |

If you introduce a new observability stream (trace IDs, peer-relay metrics, classifier decisions), follow the same shape:

1. **New env var** named `BPMUX_<DOMAIN>_LOG`
2. **No-op when unset**
3. **Append-only file writes**
4. **Timestamped lines**
5. **Silent error handling**

A future story (`docs/research/architecture/2026-04-03-...` — TBD) may consolidate these into a single `BPMUX_LOG=path[:domain1,domain2]` scheme once we have four or more. For now, one env var per sink is simpler.

## When NOT to use diag

- **User-facing messages.** If a message is intended for the user to read (e.g. "session saved", "peer connected"), surface it through the TUI's own rendering layer — the `session::broadcast::system_message(...)` path or equivalent. The diag log is for code that needs to report *to itself*, not to the user.
- **Panic / fatal errors.** If you're about to crash, `panic!` and let the runtime handle it. Do not try to diag a fatal error — the runtime will write the panic to stderr anyway, but at that point the TUI is tearing down and the frame corruption no longer matters.
- **Tests.** Unit tests don't have a TUI and can use `println!` / `eprintln!` normally. The rule is about production code that runs while the TUI is live.

## How to audit

`grep -rn "eprintln!" src/` should produce only the following expected sites (as of `adbd76e`):

- `src/bridge/mod.rs` — separate process, no TUI
- `src/main.rs` — pre-TUI startup error
- `src/lib.rs:114` — headless mode banner (TUI not active)
- `src/reload.rs` — reload message, TUI already torn down

Any new `eprintln!` outside these contexts is a regression. Add it to your PR review checklist.

## Related

- `#144` — InstrumentedStorage stderr bleed
- Commit `928cbc4` — InstrumentedStorage fix (BPMUX_STORAGE_LOG)
- Commit `adbd76e` — Background eprintln cleanup (BPMUX_LOG)
- `.claude/hooks/auto-approve-bash.sh` — BPMUX_HOOK_LOG, same shape
- [Desk-check TUI changes against a live TUI](../../-Users-ryandurling/memory/feedback_desk_check_tui.md) — designer's lesson from #144 sign-off
