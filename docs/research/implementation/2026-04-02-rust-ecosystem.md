# Rust Ecosystem Evaluation for bpmux

*Researched April 2, 2026. Crate evaluations for each bounded context in the spec.*

---

## Summary: Recommended Crate Stack

| Bounded Context | Crate | Version | Confidence |
|---|---|---|---|
| `transport/` — WebRTC | **str0m** | v0.16.2 | High — sans-I/O, pure Rust, libp2p migrated to it |
| `mcp/` — MCP server | **rmcp** (official) | v0.8.1 | High — official SDK, tokio-native, `#[tool]` macro |
| `terminal/` — TUI | **ratatui** + **crossterm** | v0.30.0 / v0.29+ | High — de facto standard, 22M downloads |
| `web/` — HTTP + WebSocket server | **axum** | v0.8.8 | High — tokio team, 270M downloads |
| `signaling/` — WebSocket client | **tokio-tungstenite** | latest | High — standard async WebSocket client |
| `session/` — message log | **serde_json** + **tokio::fs** | stable | High — JSONL is trivial with serde |
| CLI | **clap** | v4.x | High — standard CLI framework |
| Errors | **thiserror** | v1.x / v2.x | High — standard error derive |
| Encryption | **ring** or **aes-gcm** + **pbkdf2** | stable | High — well-audited crypto crates |
| Async runtime | **tokio** (multi-thread) | v1.x | High — required by axum, rmcp, crossterm |

---

## WebRTC: str0m

**Why str0m over alternatives:**
- **Sans-I/O design** — no hidden async runtime, no threading assumptions. You feed it bytes, it gives you bytes. Integrates into any tokio event loop via `tokio::select!`.
- **Pure Rust** — single binary, no FFI, no C++ toolchain.
- **Production validated** — Lookback uses it as SFU in production. libp2p migrated from webrtc-rs to str0m specifically for data channels (the exact bpmux use case).
- **Light dependency tree** — avoids the bloat of webrtc-rs.
- **Active maintenance** — steady releases through 2026, maintained by Martin Algesten (creator of `ureq`).

**Integration pattern for bpmux:**
```
loop {
    tokio::select! {
        // Receive UDP packet, feed to str0m
        Ok((bytes, addr)) = udp_socket.recv_from(&mut buf) => {
            rtc.handle_input(Input::Receive(now, Receive { source: addr, destination: local_addr, contents: (&buf[..bytes]).try_into()? }));
        }
        // str0m wants to send something
        _ = timeout => {
            // poll str0m for output, send via UDP socket
        }
    }
    // Handle str0m events (data channel open, message received, etc.)
    while let Some(event) = rtc.poll_event() {
        match event {
            Event::ChannelData(data) => { /* message received */ }
            Event::ChannelOpen(id, label) => { /* data channel opened */ }
            _ => {}
        }
    }
}
```

**Fallback:** webrtc-rs v0.20/rtc (also sans-I/O, but less production validation). Last resort: datachannel-rs (C++ FFI).

**QUIC note:** If bpmux ever drops the browser-interop requirement, QUIC (via `quinn`) would be dramatically simpler than WebRTC. No ICE, no DTLS, no SCTP layering. Worth remembering for v2+ architecture decisions.

---

## MCP: rmcp (Official Anthropic SDK)

**Why rmcp:**
- **Official** — maintained under the `modelcontextprotocol` GitHub org (Linux Foundation / Agentic AI Foundation).
- **Tokio-native** — same runtime as everything else in bpmux.
- **`#[tool]` proc macro** — define MCP tools by annotating async functions. Eliminates JSON Schema boilerplate.
- **TCP transport via `transport-async-rw`** — accepts any `tokio::io::AsyncRead + AsyncWrite`. Pass a `TcpStream` directly.
- **Handles protocol internals** — initialize handshake, capability negotiation, JSON-RPC framing, cancellation.

**Integration for bpmux:**
```rust
// Define a tool
#[tool]
async fn send_message(text: String) -> Result<String, Error> {
    // append to log, notify broadcast channel
}

// Serve over TCP
let listener = TcpListener::bind("127.0.0.1:4117").await?;
loop {
    let (stream, _) = listener.accept().await?;
    tokio::spawn(serve(stream, handler.clone()));
}
```

**Feature flags needed:** `server`, `transport-async-rw`, `macros`

**Fallback:** Direct implementation (~500-700 lines). The protocol is simple enough for a tools-only localhost server. But rmcp eliminates that effort and tracks spec changes automatically.

**Community crates:** Not recommended — fragmented, no governance advantage, higher abandonment risk.

---

## TUI: ratatui + crossterm

**Why ratatui:**
- **De facto standard** — 22M downloads, no real competitor in the Rust TUI space.
- **Immediate-mode rendering** — redraw entire frame from state each tick. Maps perfectly to bpmux's log-as-source-of-truth design.
- **Test support** — `TestBackend` renders to in-memory buffer. Assert on cell contents. Snapshot testing with `insta`.
- **v0.30 modularization** — split into `ratatui-core` and `ratatui-widgets` for better compile times and API stability.

**Async integration pattern:**
```rust
loop {
    tokio::select! {
        Some(event) = crossterm_events.next() => { handle_input(event); }
        Ok(_) = wake_rx.recv() => { /* new message, will redraw */ }
        _ = tick.tick() => { /* periodic redraw */ }
    }
    terminal.draw(|f| render_ui(f, &app_state))?;
}
```

crossterm provides `EventStream` (async, tokio-compatible) for terminal input. ratatui handles rendering. The `tokio::select!` loop multiplexes terminal input, network events, and timers.

**Relevant widgets:**
- `Paragraph` with per-line `Style` — chat messages with different colors per sender
- `Block` — pane borders and titles
- `Layout` — constraint-based splitting (chat pane / input pane / status bar)
- `tui-textarea` (ecosystem) — multi-line input widget

**Testing:** `TestBackend` + `assert_buffer` for unit tests. `ratatui-testlib` for PTY-based integration tests. Color assertions are limited.

---

## Web UI + Signaling: axum

**Why axum:**
- **Tokio team** — same maintainers as the async runtime bpmux depends on. First-class integration guaranteed.
- **Built-in WebSocket** — upgrade HTTP connections to WebSocket for signaling and live UI updates.
- **270M downloads** — the leading Rust web framework.

**Architecture for bpmux:**

One axum instance on port 4118 handles three concerns:
1. **`GET /`** — serve embedded Web UI (HTML/CSS/JS via `include_bytes!`)
2. **`GET /signal`** — WebSocket for signaling (SDP relay between peers)
3. **`GET /ws`** — WebSocket for live Web UI updates (push new messages to browser)

```rust
let app = Router::new()
    .route("/", get(serve_index))
    .route("/signal", get(signaling_ws))
    .route("/ws", get(webui_ws));
```

**What axum does NOT handle:**
- **MCP TCP server (port 4117)** — raw TCP, not HTTP. Needs separate `tokio::net::TcpListener`.
- **WebSocket client** — nodes connecting to signaling server use `tokio-tungstenite`, not axum.

**Embedded assets:** For v1 (one HTML file, one CSS, one JS), `include_bytes!` is simplest. No build step, no extra crate. If the Web UI grows, switch to `rust-embed`.

---

## Process Architecture

All components run as tokio tasks in a single process:

```
tokio::main
├── axum server (port 4118) — Web UI + signaling WebSocket
├── MCP TCP server (port 4117) — rmcp over TcpListener
├── TUI event loop — ratatui + crossterm (or headless stdout logger)
├── str0m WebRTC loop — UDP socket + str0m state machine
├── Session manager — message log (JSONL), broadcast channel (wake signals)
└── Control socket listener — Unix domain socket for `bpmux send`
```

Shared state flows through the session manager via the tokio broadcast channel. Each component reads from the log and receives wake notifications. No shared mutable state beyond `Arc<Mutex<>>` for the session state.

---

## Sources

- [str0m GitHub](https://github.com/algesten/str0m) | [crates.io](https://crates.io/crates/str0m)
- [rmcp GitHub](https://github.com/modelcontextprotocol/rust-sdk) | [crates.io](https://crates.io/crates/rmcp)
- [ratatui GitHub](https://github.com/ratatui/ratatui) | [ratatui.rs](https://ratatui.rs)
- [axum GitHub](https://github.com/tokio-rs/axum) | [crates.io](https://crates.io/crates/axum)
- [tokio-tungstenite](https://crates.io/crates/tokio-tungstenite)
- [libp2p migration to str0m](https://github.com/libp2p/rust-libp2p/issues/3659)
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [webrtc-rs blog](https://webrtc.rs/blog/)
