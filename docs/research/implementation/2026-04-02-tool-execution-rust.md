# Tool Execution in Rust for bpmux

Research on building AI tool execution (file ops, terminal, search, LSP, permissions) when bpmux wraps the Claude Messages API directly.

## 1. File Operations: Safe Read/Write

### Path Validation and Sandboxing

The core problem: user-influenced paths can escape intended directories via traversal (`../../etc/passwd`), symlink chains, or null byte injection.

**Standard library pattern** -- canonicalize + starts_with:

```rust
use std::path::Path;

fn validate_path(base: &Path, requested: &str) -> Result<PathBuf, ToolError> {
    let base = base.canonicalize()?;
    let full = base.join(requested).canonicalize()?;
    if !full.starts_with(&base) {
        return Err(ToolError::PathTraversal(requested.into()));
    }
    Ok(full)
}
```

Limitation: `canonicalize` requires the file to exist. For write-to-new-file, you need component-level validation.

**`path_jail` crate** (zero-dependency, purpose-built):

```rust
use path_jail::Jail;

let jail = Jail::new("/project/workspace")?;
let path = jail.join(user_input)?; // Blocks traversal, symlink escapes, null bytes
std::fs::write(&path, content)?;
```

- `JailedPath` type gives compile-time guarantee that a path is within the jail
- Supports files that don't exist yet (unlike raw `canonicalize`)
- Optional `secure-open` feature for TOCTOU protection via `O_NOFOLLOW`

**`cap-std` crate** (Bytecode Alliance, capability-based):

```rust
use cap_std::fs::Dir;

let workspace = Dir::open_ambient_dir("/project/workspace", cap_std::ambient_authority())?;
let contents = workspace.read_to_string("src/main.rs")?;
workspace.write("output.txt", data)?;
// Any path outside workspace is unreachable by construction
```

- No global state; sandboxing is per-`Dir` instance
- Protects against malicious concurrent modifications (race conditions)
- Cross-platform (Linux, macOS, FreeBSD, Windows)
- More invasive -- replaces `std::fs` types throughout

**Recommendation for bpmux:** Start with `path_jail` for simplicity. It validates paths at the boundary without replacing the entire filesystem API. Consider `cap-std` later if bpmux needs defense against concurrent filesystem races.

### File Read/Write Implementation

```rust
use std::fs;
use path_jail::Jail;

pub struct FileTools {
    jail: Jail,
    max_read_bytes: usize,
}

impl FileTools {
    pub fn read(&self, relative_path: &str) -> Result<String, ToolError> {
        let path = self.jail.join(relative_path)?;
        let metadata = fs::metadata(&path)?;
        if metadata.len() > self.max_read_bytes as u64 {
            return Err(ToolError::FileTooLarge(metadata.len()));
        }
        Ok(fs::read_to_string(&path)?)
    }

    pub fn write(&self, relative_path: &str, content: &str) -> Result<(), ToolError> {
        let path = self.jail.join(relative_path)?;
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(fs::write(&path, content)?)
    }
}
```


## 2. Terminal Command Execution

### Async Execution with tokio::process::Command

```rust
use tokio::process::Command;
use std::process::Stdio;

pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

pub async fn execute_command(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    timeout: Duration,
) -> Result<CommandResult, ToolError> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true) // Kill child if future is dropped
        .spawn()?;

    // Race the child against a timeout
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => Ok(CommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
            timed_out: false,
        }),
        Ok(Err(e)) => Err(ToolError::CommandFailed(e.to_string())),
        Err(_) => {
            // Timeout expired -- kill_on_drop handles cleanup
            // but we can also explicitly kill
            child.kill().await.ok();
            Ok(CommandResult {
                stdout: String::new(),
                stderr: format!("Command timed out after {}s", timeout.as_secs()),
                exit_code: None,
                timed_out: true,
            })
        }
    }
}
```

### Key Patterns

- **`kill_on_drop(true)`** -- ensures child is killed if the spawning future is cancelled (e.g., user hits Ctrl-C). This is critical for preventing orphaned processes.
- **`tokio::time::timeout`** -- race the child process against a deadline. On timeout, the child is killed via `child.kill().await`.
- **Streaming output** -- for long-running commands, take `child.stdout` and read line-by-line with `BufReader::new(stdout).lines()` instead of `wait_with_output()`.
- **`select!` for cancellation** -- use `tokio::select!` to race the command against a cancellation signal (e.g., from a oneshot channel) so the TUI can abort a running tool.

### Streaming stdout/stderr Concurrently

```rust
use tokio::io::{AsyncBufReadExt, BufReader};

let mut child = Command::new(program)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

let stdout = BufReader::new(child.stdout.take().unwrap());
let stderr = BufReader::new(child.stderr.take().unwrap());

let mut stdout_lines = stdout.lines();
let mut stderr_lines = stderr.lines();

loop {
    tokio::select! {
        line = stdout_lines.next_line() => {
            match line? {
                Some(line) => { /* send to TUI as it arrives */ },
                None => break,
            }
        }
        line = stderr_lines.next_line() => {
            match line? {
                Some(line) => { /* send to TUI */ },
                None => {},
            }
        }
    }
}

let status = child.wait().await?;
```


## 3. Code Search

### Option A: Shell Out to ripgrep

Simplest approach. ripgrep is fast, handles gitignore, binary detection, and encoding.

```rust
pub async fn grep_search(
    pattern: &str,
    path: &Path,
    file_glob: Option<&str>,
) -> Result<String, ToolError> {
    let mut cmd = Command::new("rg");
    cmd.arg("--json")       // Structured output
       .arg("--max-count").arg("50")
       .arg(pattern)
       .arg(path);

    if let Some(glob) = file_glob {
        cmd.arg("--glob").arg(glob);
    }

    let output = cmd.output().await?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
```

Pros: battle-tested, respects .gitignore, fast. Cons: external dependency on `rg` binary.

### Option B: Use the grep Crate Family Programmatically

The `grep-searcher` + `grep-regex` + `grep-matcher` crates are ripgrep's internals, usable as a library.

```rust
use grep_regex::RegexMatcher;
use grep_searcher::Searcher;
use grep_searcher::sinks::UTF8;

let matcher = RegexMatcher::new(pattern)?;
let mut matches: Vec<(u64, String)> = vec![];

Searcher::new().search_path(
    &matcher,
    path,
    UTF8(|line_number, line| {
        matches.push((line_number, line.to_string()));
        Ok(true) // continue searching
    }),
)?;
```

Important caveat from BurntSushi: the `grep` crates only handle searching. For file discovery (walking directories, respecting gitignore, filtering by type), use the `ignore` crate (also from ripgrep) or `walkdir`:

```rust
use ignore::WalkBuilder;

let walker = WalkBuilder::new(root)
    .hidden(true)          // skip hidden files
    .git_ignore(true)      // respect .gitignore
    .build();

for entry in walker {
    let entry = entry?;
    if entry.file_type().map_or(false, |ft| ft.is_file()) {
        // Run grep_searcher on this file
    }
}
```

**Recommendation for bpmux:** Start by shelling out to `rg --json`. It is simpler, avoids linking ripgrep's internals, and `rg` is nearly universally installed on developer machines. Fall back to the grep crate family if bpmux needs to work without `rg` installed.


## 4. LSP Integration from Rust

### Available Crates

**`async-lsp`** -- the most promising for bpmux. Built on Tower, supports both client and server roles with pluggable middleware.

- Provides `LanguageClient` and `LanguageServer` traits
- Async-first design (tokio-compatible)
- Uses `lsp-types` for all LSP protocol types

**`lsp-server`** -- from the rust-analyzer project. Synchronous, crossbeam-channel based. Designed for building servers, not clients.

**`lsp-client-rs`** -- a dedicated LSP client crate. Less mature but purpose-built for the client role.

**`tower-lsp` / `tower-lsp-server`** -- primarily for building LSP servers, not clients. Community fork is more active.

### How bpmux Would Talk to rust-analyzer

```rust
// Conceptual sketch using async-lsp as a client
use lsp_types::{
    InitializeParams, TextDocumentIdentifier, Position,
    GotoDefinitionParams, TextDocumentPositionParams,
};

// 1. Spawn rust-analyzer as a child process
let mut child = Command::new("rust-analyzer")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;

// 2. Wire stdin/stdout to the LSP client transport
// async-lsp provides transport adapters for this

// 3. Send initialize request
let init_result = client.initialize(InitializeParams {
    root_uri: Some(Url::from_directory_path(workspace)?),
    capabilities: ClientCapabilities::default(),
    ..Default::default()
}).await?;

// 4. Send initialized notification
client.initialized(InitializedParams {})?;

// 5. Query: go-to-definition
let def = client.goto_definition(GotoDefinitionParams {
    text_document_position_params: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: file_uri },
        position: Position { line: 10, character: 5 },
    },
    ..Default::default()
}).await?;
```

### Practical Considerations

- rust-analyzer is heavyweight (~200MB RSS). Starting it per-session may be acceptable; starting it per-query is not.
- The LSP handshake (initialize, wait for indexing) takes seconds. Keep the connection alive across tool calls.
- For bpmux's use case (code intelligence for AI), the most useful LSP methods are:
  - `textDocument/definition` -- go to definition
  - `textDocument/references` -- find all references
  - `textDocument/hover` -- type info and docs
  - `textDocument/documentSymbol` -- list symbols in a file
  - `workspace/symbol` -- search symbols across workspace
- **Alternative**: for simpler needs, `tree-sitter` provides AST parsing without a running language server. The `tree-sitter` crate works well for syntax-aware search and symbol extraction without the overhead of LSP.


## 5. Permission Model in a TUI Context

### The Challenge

bpmux uses ratatui for its TUI. Unlike a CLI where you can use `dialoguer` or simple stdin prompts, a TUI owns the terminal. Permission dialogs must be rendered as part of the TUI state machine.

### Pattern: Modal Confirmation State

```rust
enum AppState {
    Normal,
    AwaitingPermission {
        tool_name: String,
        description: String,
        // Channel to send the user's decision back to the tool executor
        response_tx: oneshot::Sender<bool>,
    },
}

// In the event loop:
match app.state {
    AppState::AwaitingPermission { .. } => {
        // Render a confirmation dialog overlay
        // Only accept 'y'/'n' or Enter/Escape
        match event {
            KeyEvent { code: KeyCode::Char('y'), .. } => {
                let tx = take_response_tx(&mut app.state);
                tx.send(true).ok();
                app.state = AppState::Normal;
            }
            KeyEvent { code: KeyCode::Char('n'), .. } |
            KeyEvent { code: KeyCode::Esc, .. } => {
                let tx = take_response_tx(&mut app.state);
                tx.send(false).ok();
                app.state = AppState::Normal;
            }
            _ => {} // Ignore other keys
        }
    }
    // ...
}
```

### TUI Confirm Dialog Libraries

- **`tui-confirm-dialog`** -- a ratatui widget specifically for yes/no confirmation dialogs
- **`tui-prompts`** -- more general prompt widgets for ratatui (text input, selection, confirmation)

### Permission Categories

Design the permission model around risk tiers:

| Tier | Examples | Behavior |
|------|----------|----------|
| Auto-allow | File read within workspace, grep/search | Execute immediately |
| Confirm once | File write, non-destructive commands (`cargo build`) | Ask once, remember for session |
| Always confirm | Shell commands with side effects, file delete | Ask every time |
| Deny | Commands outside workspace, network access, `rm -rf` | Block and report |

### Configuration

```rust
struct PermissionConfig {
    auto_allow: Vec<ToolPattern>,
    confirm_once: Vec<ToolPattern>,
    always_confirm: Vec<ToolPattern>,
    deny: Vec<ToolPattern>,
}
```

Allow users to customize via a config file, similar to how Claude Code uses `.claude/settings.json` to define allowed/denied tool patterns.


## 6. The Tool Execution Loop

### Overall Flow

```
User message
    |
    v
Messages API call (with tool definitions)
    |
    v
Response contains tool_use blocks?
  No  --> Display assistant text, done
  Yes --> For each tool_use:
            Match tool name
            Execute tool
            Collect result
          Send tool_result messages back to API
          Loop (goto Messages API call)
```

### Concrete Rust Pattern

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Tool definitions sent to the API ---

#[derive(Serialize)]
struct ToolDefinition {
    name: String,
    description: String,
    input_schema: Value, // JSON Schema
}

// --- Response types from the API ---

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Serialize)]
struct ToolResult {
    #[serde(rename = "type")]
    content_type: String, // "tool_result"
    tool_use_id: String,
    content: String,
}

// --- The agent loop ---

async fn agent_loop(
    client: &ApiClient,
    tools: &[ToolDefinition],
    executor: &ToolExecutor,
    initial_message: &str,
) -> Result<String, AgentError> {
    let mut messages: Vec<Message> = vec![
        Message::user(initial_message),
    ];

    loop {
        let response = client.create_message(&messages, tools).await?;

        // Collect text output and tool calls
        let mut assistant_text = String::new();
        let mut tool_calls: Vec<(String, String, Value)> = vec![];

        for block in &response.content {
            match block {
                ContentBlock::Text { text } => {
                    assistant_text.push_str(text);
                }
                ContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push((id.clone(), name.clone(), input.clone()));
                }
            }
        }

        // Add assistant message to history
        messages.push(Message::assistant(response.content));

        // If no tool calls, we are done
        if tool_calls.is_empty() {
            return Ok(assistant_text);
        }

        // Execute each tool and collect results
        let mut tool_results: Vec<ToolResult> = vec![];
        for (id, name, input) in tool_calls {
            let result = executor.execute(&name, &input).await?;
            tool_results.push(ToolResult {
                content_type: "tool_result".into(),
                tool_use_id: id,
                content: result,
            });
        }

        // Add tool results as a user message and loop
        messages.push(Message::tool_results(tool_results));
    }
}
```

### The Tool Executor

```rust
struct ToolExecutor {
    file_tools: FileTools,
    working_dir: PathBuf,
    command_timeout: Duration,
    permission_tx: mpsc::Sender<PermissionRequest>,
}

impl ToolExecutor {
    async fn execute(&self, name: &str, input: &Value) -> Result<String, ToolError> {
        match name {
            "read_file" => {
                let path: String = serde_json::from_value(input["path"].clone())?;
                self.file_tools.read(&path)
            }
            "write_file" => {
                let path: String = serde_json::from_value(input["path"].clone())?;
                let content: String = serde_json::from_value(input["content"].clone())?;
                self.request_permission(&format!("Write to {path}")).await?;
                self.file_tools.write(&path, &content)?;
                Ok(format!("Wrote {} bytes to {path}", content.len()))
            }
            "run_command" => {
                let command: String = serde_json::from_value(input["command"].clone())?;
                let args: Vec<String> = serde_json::from_value(
                    input.get("args").cloned().unwrap_or(Value::Array(vec![]))
                )?;
                self.request_permission(&format!("Run: {command} {}", args.join(" "))).await?;
                let result = execute_command(
                    &command,
                    &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &self.working_dir,
                    self.command_timeout,
                ).await?;
                Ok(format!(
                    "exit_code: {:?}\nstdout:\n{}\nstderr:\n{}",
                    result.exit_code, result.stdout, result.stderr
                ))
            }
            "grep_search" => {
                let pattern: String = serde_json::from_value(input["pattern"].clone())?;
                let path = input.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                grep_search(&pattern, &self.working_dir.join(path), None).await
            }
            _ => Err(ToolError::UnknownTool(name.into())),
        }
    }

    async fn request_permission(&self, description: &str) -> Result<(), ToolError> {
        let (tx, rx) = oneshot::channel();
        self.permission_tx.send(PermissionRequest {
            description: description.to_string(),
            response_tx: tx,
        }).await.map_err(|_| ToolError::PermissionDenied)?;

        match rx.await {
            Ok(true) => Ok(()),
            _ => Err(ToolError::PermissionDenied),
        }
    }
}
```

### Rust Crates for Claude API Integration

Several Rust crates wrap the Anthropic Messages API with tool_use support:

- **`misanthropy`** -- defines tools via `schemars` for JSON Schema generation. `Tool::new::<MyStruct>()` creates tool definitions from Rust types. Manual loop for tool result handling.
- **`anthropic-sdk-rust`** -- idiomatic, async-first SDK with typed tool support
- **`claude-sdk`** -- lighter-weight alternative

All use serde for serialization. The tool execution loop is not built into any of them -- you write it yourself (which is correct; the loop logic is application-specific).


## Summary of Recommended Crates

| Concern | Crate | Notes |
|---------|-------|-------|
| Path sandboxing | `path_jail` | Zero-dep, validates at boundary |
| Capability filesystem | `cap-std` | Heavier, prevents races by construction |
| Async process | `tokio` (built-in) | `kill_on_drop`, timeout via `tokio::time` |
| Code search | Shell out to `rg` | Or `grep-searcher` + `ignore` crates |
| LSP client | `async-lsp` | Tower-based, async, client+server |
| LSP types | `lsp-types` | Protocol type definitions |
| AST parsing (alt to LSP) | `tree-sitter` | Lightweight, no server process |
| TUI confirm dialog | `tui-confirm-dialog` | ratatui widget |
| Claude API | `misanthropy` or `anthropic-sdk-rust` | Tool definitions via schemars |
| JSON Schema from types | `schemars` | Derive `JsonSchema` for tool inputs |
