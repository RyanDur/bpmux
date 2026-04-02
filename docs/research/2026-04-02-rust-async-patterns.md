# Rust Async Patterns for bpmux

*Researched April 2, 2026. Practical tokio patterns for bpmux's multi-subsystem architecture.*

---

## Top-Level Architecture

bpmux runs multiple concurrent subsystems in a single tokio runtime. The recommended pattern: **spawn each subsystem as its own task, use `CancellationToken` + `TaskTracker` for graceful shutdown.**

```
tokio::main
├── CancellationToken (shared shutdown signal)
├── TaskTracker (tracks all subsystem tasks)
├── WebRTC task (str0m I/O loop)
├── MCP TCP server task (rmcp)
├── TUI task (ratatui + crossterm event loop)
├── HTTP/WebSocket task (axum — Web UI + signaling)
├── Unix socket task (control socket for bpmux send)
└── select! { ctrl_c, any subsystem failure } → token.cancel()
```

When any subsystem fails or the user presses Ctrl-C, `token.cancel()` propagates shutdown to all tasks. `tracker.wait()` ensures cleanup completes.

**Source:** [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)

## Shared State: Channels Over Mutexes

**Prefer channels between subsystems. Reserve `Arc<Mutex>` for simple shared config.**

| Pattern | When to Use | bpmux Application |
|---|---|---|
| `mpsc` + `oneshot` (actor) | State requires async updates, clear ownership boundary | Session manager (owns log, handles read/write commands) |
| `broadcast` | Event bus — multiple consumers need every event | Wake signal for new messages (TUI, peer relay, Web UI all listen) |
| `watch` | Latest-value broadcasting | Connection state (solo/connected/degraded) |
| `Arc<Mutex<T>>` | Simple shared data, short synchronous lock times | Config, session registry |

**Critical rule:** Never hold a `std::sync::MutexGuard` across an `.await`. Scope the lock so it drops before any await point. Or wrap the mutex in a struct with synchronous-only methods.

**Source:** [Tokio Shared State](https://tokio.rs/tokio/tutorial/shared-state) | [Tokio Channels](https://tokio.rs/tokio/tutorial/channels)

## str0m + Tokio Integration

str0m is sans-I/O — no threads, no async, no sockets. The `Rtc` instance is a pure state machine driven by `poll_output()` and `handle_input()`. Tokio integration:

```rust
loop {
    // Drain pending transmits first
    if let Some(transmit) = rtc.poll_transmit() {
        socket.send_to(&transmit.payload, transmit.dst).await?;
        continue;
    }

    tokio::select! {
        // str0m timeout
        _ = tokio::time::sleep_until(rtc.poll_timeout()) => {
            rtc.handle_timeout(Instant::now());
        }
        // Incoming UDP packet
        res = socket.recv_from(&mut buf) => {
            let (n, source) = res?;
            rtc.handle_input(&buf[..n], Instant::now());
        }
        // Shutdown signal
        _ = token.cancelled() => break,
        // Commands from other subsystems
        Some(cmd) = cmd_rx.recv() => {
            handle_command(&mut rtc, cmd);
        }
    }

    // Process events
    while let Some(event) = rtc.poll_event() {
        match event {
            Event::ChannelData(data) => { /* message received → append to log */ }
            Event::ChannelOpen(id, label) => { /* data channel opened */ }
            _ => {}
        }
    }
}
```

**Key insight from Firezone (production str0m user):** The sans-I/O pattern makes testing trivial — feed controlled inputs, assert on outputs, without needing a real network.

**Source:** [Firezone Sans-IO Blog](https://www.firezone.dev/blog/sans-io) | [str0m docs.rs](https://docs.rs/str0m)

## Error Handling Across Task Boundaries

`tokio::spawn` returns `JoinHandle<T>`. Awaiting it gives `Result<T, JoinError>`:
- `JoinError::Panic` — the task panicked (isolated, doesn't crash other tasks)
- `JoinError::Cancelled` — the task was aborted

If the task returns `Result`, you get the double-`?` pattern: `handle.await??`

**bpmux pattern:** When any subsystem exits (success or failure), trigger global shutdown:

```rust
tokio::select! {
    result = webrtc_handle => {
        tracing::error!("WebRTC exited: {:?}", result);
        token.cancel();
    }
    result = mcp_handle => { /* same */ }
    _ = tokio::signal::ctrl_c() => { token.cancel(); }
}
```

## Testing Async Code

**`#[tokio::test]`** for basic async tests. **`start_paused = true`** for deterministic time:

```rust
#[tokio::test(start_paused = true)]
async fn test_timeout() {
    tokio::time::sleep(Duration::from_secs(3600)).await; // completes instantly
    // Logical time advanced 1 hour, wall clock ~0ms
}
```

This pairs perfectly with str0m — which takes explicit `Instant` parameters — enabling deterministic testing of the WebRTC state machine.

**Mock I/O** via `tokio_test::io::Builder` for testing MCP and signaling handlers without real sockets.

**Source:** [Tokio Testing](https://tokio.rs/tokio/topics/testing)

## Common Pitfalls

1. **Deadlock:** Holding `MutexGuard` across `.await`. Scope locks before await points.
2. **Blocking the runtime:** `std::fs`, CPU-heavy work, `thread::sleep` inside async. Use `spawn_blocking()` or `tokio::fs`.
3. **Send + Sync bounds:** `tokio::spawn` requires `Send + 'static`. Clone or Arc-wrap shared data.
4. **Cancellation drops:** Futures dropped at `.await` when cancelled. Cleanup code after `.await` may never run. Use `scopeguard` for reliable cleanup.
5. **Future starvation in select!:** One always-ready branch starves others. Spawn long work as separate tasks.
6. **RAII guards across await:** Database connections, file handles held during suspension block the resource. Acquire, use, drop before awaiting.

## Sources

- [Tokio Tutorial — Shared State](https://tokio.rs/tokio/tutorial/shared-state)
- [Tokio Tutorial — Channels](https://tokio.rs/tokio/tutorial/channels)
- [Tokio Tutorial — Spawning](https://tokio.rs/tokio/tutorial/spawning)
- [Tokio — Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)
- [Tokio — Testing](https://tokio.rs/tokio/topics/testing)
- [Firezone — Sans-IO Blog](https://www.firezone.dev/blog/sans-io)
- [str0m docs.rs](https://docs.rs/str0m)
- [Common Mistakes with Rust Async](https://www.qovery.com/blog/common-mistakes-with-rust-async)
