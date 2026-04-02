# Story 0 + Story 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Get bpmux from an empty repo to a working solo chat — a TUI where a human can type messages, see them in a scrollable chat pane, and have them persist across restarts.

**Architecture:** A Rust binary using `clap` for CLI, `ratatui` + `crossterm` for the TUI, `serde_json` for JSONL message persistence, and `tokio` for the async runtime. Messages are appended to a JSONL log file. A `tokio::sync::broadcast` channel carries wake signals so the TUI redraws when new messages arrive. The TUI uses immediate-mode rendering — full redraw from state each tick.

**Tech Stack:** Rust (stable), tokio, clap, ratatui, crossterm, serde, serde_json, chrono, dirs, uuid

---

## Story Roadmap (what comes after this plan)

| Story | Value | Key Tech |
|---|---|---|
| **0 + 1 (this plan)** | Solo chat with persistence | clap, ratatui, serde_json, tokio |
| 2 | AI joins the conversation | rmcp (MCP server) |
| 3 | Web UI for Desktop users | axum, embedded HTML, WebSocket |
| 4 | Zero-terminal session management | MCP session tools |
| 5 | Two developers connect P2P | str0m (WebRTC), signaling server |
| 6 | Encrypted, authenticated sessions | AES-256-GCM, PBKDF2, session tokens |
| 7 | Token-aware context delivery | Read cursor, bounded windows, config |
| 8 | LLM trust and safety | Trust levels, injection detection |
| 9 | CI/CD pipeline | GitHub Actions, cross-platform builds |

Each future story gets its own implementation plan when the preceding stories are complete.

---

## File Structure

```
bpmux/
├── Cargo.toml                    # Package config, dependencies, clippy lints
├── Cargo.lock                    # Committed (binary, not library)
├── src/
│   ├── main.rs                   # Thin wrapper: parse args, create runtime, dispatch
│   ├── lib.rs                    # Re-exports bounded contexts
│   ├── session/
│   │   ├── mod.rs                # Re-exports: Session, Message, SessionConfig
│   │   ├── message.rs            # Message struct, serialization, JSONL append/read
│   │   └── log.rs                # Log file management (open, append, read, path resolution)
│   └── terminal/
│       ├── mod.rs                # Re-exports: run_tui
│       ├── app.rs                # App state (messages, input buffer, scroll position)
│       ├── ui.rs                 # Rendering (chat pane, input pane, status bar)
│       └── event.rs              # Event handling (key input, wake signals)
├── tests/
│   └── integration/
│       └── session_test.rs       # Integration: write messages, read them back
├── .github/
│   └── workflows/
│       └── ci.yml                # CI: fmt, clippy, test
├── .git/
│   └── hooks/
│       └── pre-push              # Quality gate: fmt + clippy + test
├── CLAUDE.md                     # AI collaboration guide
├── AGENT_WORKFLOW.md             # (already exists)
├── LICENSE                       # BSL
├── .gitignore                    # (already exists, update for Rust)
└── docs/                         # (already exists)
```

---

## Task 1: Rust Toolchain and Cargo Project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Modify: `.gitignore`

- [ ] **Step 1: Install Rust toolchain**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup --version
cargo --version
rustc --version
```

Expected: Rust stable installed, `cargo` and `rustc` available.

- [ ] **Step 2: Initialize Cargo project**

```bash
cd /Users/ryandurling/IdeaProjects/bpmux
cargo init --name bpmux
```

This creates `Cargo.toml` and `src/main.rs`. Verify:

```bash
cargo build
cargo run
```

Expected: "Hello, world!" printed.

- [ ] **Step 3: Configure Cargo.toml with dependencies and lints**

Replace the generated `Cargo.toml` with:

```toml
[package]
name = "bpmux"
version = "0.1.0"
edition = "2024"
description = "P2P group chat for AI pair programming sessions"
license = "BUSL-1.1"

[dependencies]
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = { version = "0.29", features = ["event-stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
dirs = "6"
uuid = { version = "1", features = ["v4"] }
thiserror = "2"
futures = "0.3"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
dbg_macro = "deny"
todo = "warn"
```

- [ ] **Step 4: Create thin main.rs**

Replace `src/main.rs` with:

```rust
use std::process;

fn main() {
    if let Err(e) = bpmux::run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
```

- [ ] **Step 5: Create lib.rs with placeholder**

Create `src/lib.rs`:

```rust
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("bpmux v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
```

- [ ] **Step 6: Update .gitignore for Rust**

Add to `.gitignore`:

```
/target/
```

- [ ] **Step 7: Verify build and lints**

```bash
cargo build
cargo clippy -- -D warnings
cargo fmt --check
cargo run
```

Expected: Clean build, no clippy warnings, formatted, prints version.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/lib.rs .gitignore
git commit -m "Initialize Rust project with dependencies and clippy lints"
```

---

## Task 2: BSL License and CLAUDE.md

**Files:**
- Create: `LICENSE`
- Create: `CLAUDE.md`

- [ ] **Step 1: Add BSL license**

Create `LICENSE` with the Business Source License 1.1 text. The key fields:

```
Business Source License 1.1

Licensor: Ryan Durling
Licensed Work: bpmux
Change Date: Four years from each release date
Change License: Apache License, Version 2.0

The Licensed Work is provided under the terms of the Business Source License 1.1.
```

Note: The full BSL 1.1 text is available at https://mariadb.com/bsl11/. Copy the full legal text and fill in the fields above.

- [ ] **Step 2: Create CLAUDE.md**

Create `CLAUDE.md`:

```markdown
# bpmux

P2P group chat for AI pair programming sessions. Prevents cognitive and intent debt by capturing AI reasoning in a persistent, transparent shared environment.

## Commands

```bash
cargo build              # Build
cargo test               # Run all tests
cargo clippy -- -D warnings  # Lint (warnings are errors)
cargo fmt --check        # Check formatting
cargo run                # Run bpmux
```

## Architecture

Organized by bounded context (App Continuum), not by technical layer:

- `src/session/` — Session state, message log, JSONL persistence
- `src/terminal/` — TUI rendering, input handling, chat display

## Code Style

- **Error handling:** `Result<T, E>` everywhere. No `unwrap()` or `expect()` in production. `thiserror` for custom errors. `?` for propagation.
- **Immutability by default.** Only use `mut` when necessary.
- **Iterators over loops.** Use `.map()`, `.filter()`, `.collect()`.
- **No `unsafe` code.** `#![deny(unsafe_code)]` at crate root.
- **Tests as documentation.** Test names describe behavior: `sends_message_to_log`, not `test_send`.
- **Comments for why, not what.** If code needs a comment to explain what it does, refactor.

## Testing

- TDD: outside-in, classicist (real collaborators over mocks)
- Unit tests in same file (`#[cfg(test)]` module)
- Integration tests in `tests/`
- `cargo test` must pass before push

## Working Agreement

See `AGENT_WORKFLOW.md` for how we collaborate. It is our process, not a bpmux feature.
```

- [ ] **Step 3: Commit**

```bash
git add LICENSE CLAUDE.md
git commit -m "Add BSL license and CLAUDE.md"
```

---

## Task 3: CI Workflow and Pre-Push Hook

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.git/hooks/pre-push`

- [ ] **Step 1: Create CI workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  workflow_call:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Format check
        run: cargo fmt --check
      - name: Clippy
        run: cargo clippy -- -D warnings
      - name: Test
        run: cargo test
      - name: Audit
        run: |
          cargo install cargo-audit
          cargo audit
```

- [ ] **Step 2: Create pre-push hook**

Create `.git/hooks/pre-push`:

```bash
#!/bin/bash
set -euo pipefail

echo "Running pre-push quality gates..."
cargo fmt --check && cargo clippy -- -D warnings && cargo test
echo "All checks passed."
```

Make it executable:

```bash
chmod +x .git/hooks/pre-push
```

- [ ] **Step 3: Verify pre-push hook**

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "Add CI workflow and pre-push quality gates"
```

Note: `.git/hooks/pre-push` is not tracked by git. Document in CLAUDE.md that developers need to create it manually, or add a setup script.

---

## Task 4: Message Type and JSONL Persistence

**Files:**
- Create: `src/session/mod.rs`
- Create: `src/session/message.rs`
- Create: `src/session/log.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing test for Message serialization**

Create `src/session/message.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub from: String,
    pub text: String,
    pub ts: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_message_to_json() {
        let msg = Message {
            from: "ryan:human".to_string(),
            text: "hello world".to_string(),
            ts: DateTime::parse_from_rfc3339("2026-04-02T12:00:00Z")
                .expect("test timestamp")
                .with_timezone(&Utc),
        };

        let json = serde_json::to_string(&msg).expect("serialize");
        let parsed: Message = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed, msg);
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

```bash
cargo test session::message::tests::serializes_message_to_json
```

Expected: PASS (the struct and derive macros are defined in the same step — this is the "test and implementation together" case for pure data types).

- [ ] **Step 3: Write failing test for log append and read**

Create `src/session/log.rs`:

```rust
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::session::message::Message;

#[derive(Error, Debug)]
pub enum LogError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn log_path(session_dir: &Path) -> PathBuf {
    session_dir.join("messages-current.jsonl")
}

pub fn append_message(session_dir: &Path, message: &Message) -> Result<(), LogError> {
    let path = log_path(session_dir);
    let mut line = serde_json::to_string(message)?;
    line.push('\n');

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

pub fn read_messages(session_dir: &Path) -> Result<Vec<Message>, LogError> {
    let path = log_path(session_dir);
    if !path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&path)?;
    let messages: Result<Vec<Message>, _> = content
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line))
        .collect();

    Ok(messages?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[test]
    fn appends_and_reads_messages() {
        let dir = TempDir::new().expect("temp dir");

        let msg1 = Message {
            from: "ryan:human".to_string(),
            text: "first message".to_string(),
            ts: Utc::now(),
        };
        let msg2 = Message {
            from: "ryan:human".to_string(),
            text: "second message".to_string(),
            ts: Utc::now(),
        };

        append_message(dir.path(), &msg1).expect("append 1");
        append_message(dir.path(), &msg2).expect("append 2");

        let messages = read_messages(dir.path()).expect("read");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "first message");
        assert_eq!(messages[1].text, "second message");
    }

    #[test]
    fn reads_empty_when_no_log_exists() {
        let dir = TempDir::new().expect("temp dir");
        let messages = read_messages(dir.path()).expect("read");
        assert!(messages.is_empty());
    }
}
```

- [ ] **Step 4: Add tempfile to dev-dependencies**

Add to `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: Create session module**

Create `src/session/mod.rs`:

```rust
pub mod log;
pub mod message;

pub use log::{append_message, read_messages, LogError};
pub use message::Message;
```

Update `src/lib.rs`:

```rust
#![deny(unsafe_code)]

pub mod session;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("bpmux v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test
```

Expected: All tests pass (serialization, append/read, empty read).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/lib.rs src/session/
git commit -m "Add Message type and JSONL log persistence"
```

---

## Task 5: Session Directory Management

**Files:**
- Modify: `src/session/mod.rs`
- Modify: `src/session/log.rs`

- [ ] **Step 1: Write failing test for session directory creation**

Add to `src/session/log.rs`:

```rust
pub fn create_session_dir() -> Result<PathBuf, LogError> {
    let base = dirs::home_dir()
        .ok_or_else(|| LogError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "home directory not found",
        )))?
        .join(".bpmux")
        .join("sessions");

    let session_id = uuid::Uuid::new_v4().to_string();
    let session_dir = base.join(&session_id);
    std::fs::create_dir_all(&session_dir)?;

    // Create/update "current" symlink
    let current = base.join("current");
    if current.exists() || current.is_symlink() {
        std::fs::remove_file(&current)?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&session_dir, &current)?;

    Ok(session_dir)
}
```

And the test:

```rust
#[test]
fn creates_session_directory() {
    let dir = create_session_dir().expect("create session");
    assert!(dir.exists());
    assert!(dir.is_dir());

    // Clean up
    std::fs::remove_dir_all(dir.parent().expect("parent").parent().expect("base"))
        .expect("cleanup");
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test session::log::tests::creates_session_directory
```

Expected: PASS.

- [ ] **Step 3: Re-export in mod.rs**

Update `src/session/mod.rs`:

```rust
pub mod log;
pub mod message;

pub use log::{append_message, create_session_dir, read_messages, LogError};
pub use message::Message;
```

- [ ] **Step 4: Run all tests**

```bash
cargo test
```

Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add src/session/
git commit -m "Add session directory creation with current symlink"
```

---

## Task 6: App State

**Files:**
- Create: `src/terminal/mod.rs`
- Create: `src/terminal/app.rs`

- [ ] **Step 1: Create App struct with tests**

Create `src/terminal/app.rs`:

```rust
use crate::session::Message;
use chrono::Utc;

pub struct App {
    pub messages: Vec<Message>,
    pub input: String,
    pub scroll_offset: u16,
    pub peer_name: String,
    pub session_id: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(peer_name: String, session_id: String, messages: Vec<Message>) -> Self {
        Self {
            messages,
            input: String::new(),
            scroll_offset: 0,
            peer_name,
            session_id,
            should_quit: false,
        }
    }

    pub fn submit_message(&mut self) -> Option<Message> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }

        if text == "/quit" {
            self.should_quit = true;
            return None;
        }

        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text,
            ts: Utc::now(),
        };

        self.messages.push(msg.clone());
        self.input.clear();
        Some(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn submits_message_and_clears_input() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello world".to_string();

        let msg = app.submit_message().unwrap();

        assert_eq!(msg.from, "ryan:human");
        assert_eq!(msg.text, "hello world");
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn ignores_empty_input() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "   ".to_string();

        let msg = app.submit_message();
        assert!(msg.is_none());
    }

    #[test]
    fn quit_command_sets_flag() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/quit".to_string();

        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.should_quit);
    }
}
```

- [ ] **Step 2: Create terminal module**

Create `src/terminal/mod.rs`:

```rust
pub mod app;

pub use app::App;
```

Update `src/lib.rs`:

```rust
#![deny(unsafe_code)]

pub mod session;
pub mod terminal;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("bpmux v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

Expected: All pass including the new App tests.

- [ ] **Step 4: Commit**

```bash
git add src/terminal/ src/lib.rs
git commit -m "Add App state with message submission and quit command"
```

---

## Task 7: TUI Rendering

**Files:**
- Create: `src/terminal/ui.rs`
- Modify: `src/terminal/mod.rs`

- [ ] **Step 1: Write the rendering function with test**

Create `src/terminal/ui.rs`:

```rust
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::terminal::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(3),       // chat pane
        Constraint::Length(3),    // input pane
        Constraint::Length(1),    // status bar
    ])
    .split(frame.area());

    // Chat pane
    let messages: Vec<Line> = app
        .messages
        .iter()
        .map(|msg| {
            let time = msg.ts.format("%H:%M:%S").to_string();
            Line::from(vec![
                Span::styled(
                    format!("[{time}] "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{}: ", msg.from),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&msg.text),
            ])
        })
        .collect();

    let chat = Paragraph::new(messages)
        .block(Block::default().borders(Borders::ALL).title(" bpmux "));
    frame.render_widget(chat, chunks[0]);

    // Input pane
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" > "));
    frame.render_widget(input, chunks[1]);

    // Status bar
    let status = Paragraph::new(Line::from(vec![
        Span::styled(" solo", Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::raw(&app.session_id),
    ]));
    frame.render_widget(status, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Message;
    use chrono::Utc;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new("ryan".to_string(), "test-id".to_string(), vec![
            Message {
                from: "ryan:human".to_string(),
                text: "hello".to_string(),
                ts: Utc::now(),
            },
        ]);

        terminal
            .draw(|frame| render(frame, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("hello"));
        assert!(content.contains("ryan:human"));
        assert!(content.contains("solo"));
    }

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut s = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                s.push(buffer.cell((x, y)).map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')));
            }
            s.push('\n');
        }
        s
    }
}
```

- [ ] **Step 2: Update terminal module**

Update `src/terminal/mod.rs`:

```rust
pub mod app;
pub mod ui;

pub use app::App;
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

Expected: All pass, including the TUI rendering test.

- [ ] **Step 4: Commit**

```bash
git add src/terminal/
git commit -m "Add TUI rendering with chat pane, input, and status bar"
```

---

## Task 8: TUI Event Loop

**Files:**
- Create: `src/terminal/event.rs`
- Modify: `src/terminal/mod.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create the event loop**

Create `src/terminal/event.rs`:

```rust
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use tokio::sync::broadcast;

use crate::session;
use crate::terminal::app::App;
use crate::terminal::ui;

pub fn run_tui(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    session_dir: &Path,
    _wake_rx: &mut broadcast::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Enter => {
                        if let Some(msg) = app.submit_message() {
                            session::append_message(session_dir, &msg)?;
                        }
                        if app.should_quit {
                            return Ok(());
                        }
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Esc => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }
}
```

- [ ] **Step 2: Update terminal module**

Update `src/terminal/mod.rs`:

```rust
pub mod app;
pub mod event;
pub mod ui;

pub use app::App;
pub use event::run_tui;
```

- [ ] **Step 3: Wire up lib.rs with CLI and session**

Replace `src/lib.rs`:

```rust
#![deny(unsafe_code)]

pub mod session;
pub mod terminal;

use clap::Parser;
use tokio::sync::broadcast;

#[derive(Parser)]
#[command(name = "bpmux", version, about = "P2P group chat for AI pair programming")]
struct Cli {
    /// Your name in the session
    #[arg(long, default_value = "dev")]
    name: String,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Create or resume session
    let session_dir = session::create_session_dir()?;
    let session_id = session_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Load existing messages
    let messages = session::read_messages(&session_dir)?;

    // Create app state
    let mut app = terminal::App::new(cli.name, session_id, messages);

    // Wake channel (unused in Story 1, but wired for Story 2)
    let (_wake_tx, mut wake_rx) = broadcast::channel::<()>(16);

    // Run TUI
    let mut terminal = ratatui::init();
    let result = terminal::run_tui(&mut terminal, &mut app, &session_dir, &mut wake_rx);
    ratatui::restore();

    result
}
```

- [ ] **Step 4: Update main.rs**

Replace `src/main.rs`:

```rust
use std::process;

fn main() {
    if let Err(e) = bpmux::run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
```

- [ ] **Step 5: Build and run**

```bash
cargo build
cargo run -- --name ryan
```

Expected: TUI opens with chat pane, input pane, and status bar showing "solo" and the session ID. Type a message, press Enter, see it appear in the chat. Press Esc to quit.

- [ ] **Step 6: Verify persistence**

```bash
cargo run -- --name ryan
```

Type a message, press Esc. Run again:

```bash
cargo run -- --name ryan
```

Expected: Previous messages are visible (loaded from the JSONL log). Note: this creates a NEW session each time. Resuming an existing session is a follow-up task.

- [ ] **Step 7: Run all tests and lints**

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add src/ Cargo.toml
git commit -m "Wire up TUI event loop with session persistence — Story 1 complete"
```

---

## Self-Review Checklist

**Spec coverage:**
- ✅ `cargo build` succeeds (Task 1)
- ✅ `cargo test` runs (Task 4, 5, 6, 7)
- ✅ `cargo clippy` and `cargo fmt` pass (Task 1, 3)
- ✅ Pre-push hook gates quality (Task 3)
- ✅ CLAUDE.md provides guidance (Task 2)
- ✅ BSL license (Task 2)
- ✅ TUI with chat pane, input pane, status bar (Task 7)
- ✅ Human can type and see messages (Task 8)
- ✅ Messages persisted to JSONL (Task 4, 8)
- ✅ Status bar shows "solo" (Task 7)

**Not yet covered (future stories):**
- ❌ Restarting and reconnecting to SAME session (Story 1 creates new session each time — session resume is a follow-up)
- ❌ MCP server (Story 2)
- ❌ Web UI (Story 3)
- ❌ WebRTC (Story 5)
- ❌ Encryption (Story 6)

**Placeholder scan:** No TBD, TODO, or "implement later" found. All code is complete.

**Type consistency:** `Message`, `App`, `LogError` used consistently across all tasks. `session_dir: &Path` used everywhere for session directory references.
