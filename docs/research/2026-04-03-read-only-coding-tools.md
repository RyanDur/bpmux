# Read-Only Coding Tools: The First Compelling Upgrade

**Hypothesis:** Read-only tools (file read, grep, glob, git operations) are the first upgrade that transforms a chat bot into a useful coding companion.

---

## 1. What Read-Only Tools Exist?

### Core File System Tools
- **File read** — Read file contents with line numbers, pagination for large files
- **Grep/search** — Regex pattern matching across files (ripgrep-style)
- **Glob/find** — File pattern matching (`**/*.rs`, `src/**/*.ts`)
- **Directory listing** — `ls`, tree structure navigation

### Git Operations (All Read-Only)
- **git log** — Commit history, author, date, message
- **git diff** — Compare branches, commits, staged/unstaged changes
- **git blame** — Line-by-line attribution
- **git show** — Inspect specific commits
- **git branch** — List branches, track divergence
- **git status** — Working tree state (read, not modify)

### Language Intelligence
- **LSP hover** — Type information, documentation on any symbol
- **LSP go-to-definition** — Jump to where something is defined
- **LSP find-references** — Every call site for a function/type
- **LSP document-symbols** — Outline of a file's structure
- **LSP workspace-symbols** — Search symbols across the project
- **Tree-sitter parsing** — Concrete syntax trees without compilation
- **AST analysis** — Structural code understanding (function signatures, call graphs)

### Build/CI Artifacts (Read-Only)
- **Test results** — Read pass/fail output without running tests
- **CI status** — Check pipeline results via API
- **Compiler errors** — Read error output, reason about fixes
- **Coverage reports** — Read existing coverage data
- **Dependency lockfiles** — Analyze dependency trees from Cargo.lock, package-lock.json

### External Context
- **Documentation lookup** — Read docs, man pages, API references
- **Issue trackers** — Read GitHub issues, PR descriptions, comments
- **PR diffs** — Read what changed in a pull request

---

## 2. What Can You Do With Just Reading?

### High-Value Read-Only Activities

**Code review** — Read diffs, understand context, identify bugs, suggest improvements. This is exactly what CodeRabbit, Sourcery, and other review bots do. They never write code; they analyze and comment.

**Architecture analysis** — Read all files, understand module boundaries, identify coupling. Map dependency relationships. Answer "how does data flow from X to Y?" by tracing through source.

**Bug analysis** — Read stack traces, read the relevant code, reason about what went wrong. The AI cannot run the code, but it can read it and think. This covers a surprising amount of debugging.

**"What does this function do?"** — The most common developer question. Read the function, its callers, its tests, and explain it. No writes needed.

**Dependency audit** — Read Cargo.toml/package.json, read lockfiles, check for known vulnerabilities by cross-referencing advisory databases. Flag outdated or suspicious dependencies.

**Security scanning (static)** — Read source code and identify patterns: SQL injection, XSS, hardcoded secrets, insecure defaults. No execution needed.

**Test coverage analysis** — Read existing coverage reports. Identify untested code paths by reading source and comparing to test files.

**Onboarding acceleration** — A new developer asks "where does authentication happen?" The AI reads the codebase and answers. One team reported new engineers went from weeks to days for their first PR after deploying a repo-aware chatbot connected to architecture docs. (Source: Collin Wilkins, "AI-Assisted Coding in 2025")

**Codebase Q&A** — "Which files import this module?" "What's the error handling strategy?" "Is there a pattern for X?" All answerable by reading.

**Refactoring planning** — Read the code, identify what needs to change and where, produce a plan. The human executes. This is the "architect" pattern.

### The Surprising Breadth

The read-only tool set covers a large percentage of what developers actually ask AI assistants. Most interactions with coding AI are questions, not commands. "What does this do?" "Why is this failing?" "How should I approach X?" These are all read-only operations.

---

## 3. What CAN'T You Do Without Write Tools?

### The Hard Boundary

**File creation and modification** — Cannot create new files, edit existing files, add functions, fix bugs (in the code itself). The AI can tell you what to change, but it cannot change it.

**Command execution** — Cannot run tests, build the project, start servers, install dependencies. Cannot verify that suggested changes actually work.

**Refactoring execution** — Can plan a rename-across-codebase, but cannot execute it. The human must do every file edit.

**Code generation** — Cannot scaffold a new module, generate boilerplate, write tests. Can describe what the test should look like, but not write it.

**Git writes** — Cannot commit, push, create branches, merge. The human does all version control operations.

**Environment modification** — Cannot modify configs, set environment variables, install tools.

### The Frustration Gradient

The frustration of read-only increases with task complexity:

| Task | Read-Only Experience |
|------|---------------------|
| "What does this function do?" | Perfect. No writes needed. |
| "Find the bug in this code" | Good. AI reads and reasons. Human fixes. |
| "Rename this function everywhere" | Annoying. AI finds all 15 locations. Human edits each one. |
| "Add error handling to this module" | Frustrating. AI describes the changes. Human types them all. |
| "Scaffold a new service" | Useless. The entire task is writing. |

The boundary is clear: **read-only is compelling for understanding, frustrating for doing.**

---

## 4. Aider's Read-Only Mode

### How It Works

Aider distinguishes between two file states:
- **`/add`** — Files the AI can read AND edit
- **`/read-only`** — Files the AI can read but NOT edit

From the command line: `aider --read CONVENTIONS.md` or in-chat: `/read-only path/to/file`

Source: [Aider in-chat commands documentation](https://aider.chat/docs/usage/commands.html)

### Use Cases for Read-Only Files

1. **Convention files** — Load `CONVENTIONS.md` as read-only context. The AI follows the conventions when editing other files.
2. **Reference implementations** — "Here's how we did it in module A. Now do the same pattern in module B." Module A is read-only context.
3. **API contracts** — Load interface definitions as read-only. The AI implements against them without modifying the contract.
4. **Test files as specs** — Load failing tests as read-only. The AI reads the test and writes the implementation to make it pass.
5. **Library code** — Load third-party code as context without risking modification.

### Is Read-Only Mode Useful On Its Own?

Not as Aider's primary mode. Aider is fundamentally a write-oriented tool. The `/read-only` command exists to provide context for writes, not as a standalone feature. If you only used `/read-only` with Aider, you'd just have an expensive chatbot that can see your files.

However, the existence of `/read-only` validates that **read-only context is essential to good writes**. The AI writes better code when it can read more of the codebase. This suggests read-only tools are a prerequisite for everything else.

Source: [Aider documentation](https://aider.chat/docs/), [GitHub issue #941 on read-only files](https://github.com/paul-gauthier/aider/issues/941)

---

## 5. Code Review Bots: The Read-Only Precedent

### The Landscape (2025-2026)

Code review bots are the most mature read-only coding AI products. They read PR diffs, analyze code, and leave comments. They never modify code.

**Major players:**
- **CodeRabbit** — 2M+ connected repos, 13M+ PRs reviewed. The market leader.
- **Sourcery** — Python-focused, lower noise than CodeRabbit.
- **Macroscope** — Benchmarks well on bug detection.
- **Cursor Bugbot** — Integrated with Cursor IDE.
- **Greptile** — Codebase-aware reviews.
- **Graphite Diamond** — Newer entrant.

Source: [State of AI Code Review Tools in 2025](https://www.devtoolsacademy.com/blog/state-of-ai-code-review-tools-2025/)

### Bug Detection Rates (Macroscope 2025 Benchmark)

| Tool | Bug Detection Rate |
|------|--------------------|
| Macroscope | 48% |
| CodeRabbit | 46% |
| Cursor Bugbot | 42% |

These tools detect nearly half of real-world production bugs through read-only analysis alone. That is significant.

Source: [DevTools Academy benchmark](https://www.devtoolsacademy.com/blog/state-of-ai-code-review-tools-2025/)

### Developer Experience: Mixed

**What developers like:**
- Catches real bugs (null pointers, unused variables, logic errors)
- Finds issues humans miss in review fatigue
- Provides context and reasoning with suggestions
- Learns from feedback over time
- One developer described CodeRabbit as "the one AI tool I wouldn't want to be without"

**What developers dislike:**
- Noise. CodeRabbit is the most "talkative" tool, leaving many comments per PR
- Overzealous security flagging (false positives on context-safe patterns)
- Endless refinement loops — one developer reported the AI reviewing its own suggestion, concluding the code was "too difficult to understand," and suggesting reverting to the original
- About 72% of CodeRabbit findings were relevant in one analysis, with roughly half of those bringing genuine value
- Developers describe it as "a mixed bag, perhaps 50-50 useful and useless"

**The consensus:** Treat AI review as a junior developer's suggestions, not an authority. Still essential to have human review.

Sources: [Elio Struyf's AI Code Review Journey](https://www.eliostruyf.com/ai-code-review-journey-copilot-coderabbit-macroscope/), [DeployHQ's CodeRabbit experience](https://www.deployhq.com/blog/ai-code-review-before-you-deploy-our-experience-with-coderabbit), [Sourcery vs CodeRabbit comparison](https://www.sourcery.ai/comparisons/coderabbit-alternative)

### What This Proves

Code review bots demonstrate that read-only AI is a viable, valuable product category. Millions of developers use them. They catch real bugs. They are noisy but net-positive. The key constraint: they analyze what humans wrote. They cannot fix what they find.

---

## 6. The "Architect" Pattern

### The Concept

An AI that reads everything but writes nothing. It understands the codebase, answers questions, spots issues, suggests approaches. The human does all the typing.

This is what Maximiliano Contieri calls "Force Read-Only Planning" — make the AI be your architect before letting it be your builder.

Source: [AI Coding Tip 003 — Force Read-Only Planning](https://dev.to/mcsee/ai-coding-tip-003-force-read-only-planning-1d1m)

### Tools That Support This Pattern

| Tool | Read-Only Mode |
|------|----------------|
| GitHub Copilot | Ask Mode |
| Cursor | Normal/Ask Mode |
| Aider | Ask/Help Mode |
| Windsurf | Chat Mode |
| Cline | Plan Mode |
| Claude Code | Explore agent (internal read-only subagent) |

Source: [Contieri's article](https://dev.to/mcsee/ai-coding-tip-003-force-read-only-planning-1d1m)

### Is It Viable As a Standalone Product?

**Arguments for:**
- Onboarding value is proven (weeks to days for first PR)
- Code review bots prove read-only AI has a market
- Reduces the "blast radius" of AI mistakes — if it can't write, it can't break things
- Lower trust barrier — users don't need to worry about AI modifying code
- Cheaper to implement (no sandboxing, no undo, no conflict resolution)
- Works with any editor/IDE since it doesn't need editor integration

**Arguments against:**
- Frustration builds when the AI identifies a bug but can't fix it
- Copy-pasting AI suggestions into code is tedious and error-prone
- The market is moving toward full agentic autonomy (Claude Code, Codex, Devin)
- Developers who want AI help usually want hands-on help, not advice
- "Architect without hands" feels like a tease

**The honest assessment:** Read-only is a compelling first step but not a destination. It's where you start (can the AI even understand my code?) before adding write capabilities. The value is real but the frustration is also real. The upgrade path matters — users will want writes eventually.

### Claude Code's Internal Architecture Validates This

Claude Code has a read-only subagent called "Explore" that can use Glob, Grep, Read, and limited Bash commands to navigate codebases but is strictly prohibited from creating or modifying files. The existence of a dedicated read-only agent inside a write-capable tool suggests that read-only exploration is a distinct, valuable capability worth isolating.

Source: [Claude Code Tool System Explained](https://callsphere.tech/blog/claude-code-tool-system-explained), [AI Free API guide](https://www.aifreeapi.com/en/posts/claude-code-tool-search)

---

## 7. Implementation in Rust

### File System Reading

| Crate | Purpose | Notes |
|-------|---------|-------|
| `std::fs` | Basic file reading | Built-in, no dependency needed |
| `ignore` | .gitignore-aware directory walking | From BurntSushi (ripgrep author). Respects .gitignore, .ignore, .git/info/exclude. The standard choice. |
| `glob` | Unix shell-style pattern matching | Simple, stable, well-known |
| `globset` | Multi-pattern glob matching | Converts globs to regex, faster for matching many patterns at once |
| `walkdir` | Recursive directory traversal | Simple, fast, pairs well with `ignore` |

Sources: [ignore crate on crates.io](https://crates.io/crates/ignore), [ignore docs on docs.rs](https://docs.rs/ignore), [glob crate](https://crates.io/crates/glob), [globset crate](https://crates.io/crates/globset)

### Git Operations

| Crate | Purpose | Notes |
|-------|---------|-------|
| `gix` (gitoxide) | Pure Rust git implementation | Full-featured. Supports log, diff, blame, status, branch listing. Read-only operations work well. Trust-aware (won't run untrusted executables). Actively maintained by GitoxideLabs. |
| `git2` (libgit2 bindings) | C-based git via FFI | More mature API but requires C library. Heavier dependency. |

The `gix` crate is the better choice for a Rust-native project. It provides read-only repository access, commit graph traversal, and diff computation without shelling out to git.

Sources: [gitoxide on GitHub](https://github.com/GitoxideLabs/gitoxide), [gix on docs.rs](https://docs.rs/gix/latest/gix/), [gix on crates.io](https://crates.io/crates/gitoxide)

### Code Parsing and Analysis

| Crate | Purpose | Notes |
|-------|---------|-------|
| `tree-sitter` | Incremental parsing, syntax trees | Multi-language. Concrete syntax trees. Incremental updates in <1ms after initial parse. Language grammars as separate crates (`tree-sitter-rust`, `tree-sitter-typescript`, etc.). |
| `syn` | Rust source code parsing | Rust-only. Full AST with span information. Primarily for proc macros but usable for analysis. |
| `ra_ap_syntax` | rust-analyzer's syntax library | Rust-only. The most complete Rust parser outside rustc. CST and AST layers. |
| `rust-code-analysis` (Mozilla) | Multi-language code metrics | Based on tree-sitter. Computes complexity, lines of code, and other metrics. |

For multi-language support, `tree-sitter` is the clear choice. For Rust-specific deep analysis, `ra_ap_syntax` provides the richest model.

Sources: [tree-sitter crate](https://crates.io/crates/tree-sitter), [tree-sitter docs](https://docs.rs/tree-sitter), [syn on GitHub](https://github.com/dtolnay/syn), [ra_ap_syntax docs](https://docs.rs/ra_ap_syntax/latest/ra_ap_syntax/), [rust-code-analysis on GitHub](https://github.com/mozilla/rust-code-analysis)

### Search (grep-like)

| Crate | Purpose | Notes |
|-------|---------|-------|
| `grep` | ripgrep's search library | The actual engine behind ripgrep. Line-oriented search with regex. |
| `regex` | Regular expressions | Fast, safe, no backtracking. Foundation for text search. |
| `aho-corasick` | Multi-pattern string matching | Fast simultaneous search for many literal patterns. |

Sources: [regex crate](https://crates.io/crates/regex), [grep crate](https://crates.io/crates/grep)

### LSP Client

| Crate | Purpose | Notes |
|-------|---------|-------|
| `tower-lsp` | LSP server framework | For building an LSP server, not consuming one. |
| `lsp-types` | LSP type definitions | Protocol types for both client and server use. |
| Direct process spawn | Communicate with existing LSP servers | Spawn rust-analyzer, typescript-language-server, etc. and communicate via JSON-RPC over stdio. |

For consuming LSP data from existing language servers, the pragmatic approach is to spawn the language server as a subprocess and communicate via the protocol using `lsp-types` for serialization.

Sources: [tower-lsp on GitHub](https://github.com/ebkalderon/tower-lsp), [tower-lsp docs](https://docs.rs/tower-lsp/)

---

## 8. Synthesis

### The Hypothesis Holds

Read-only tools are the first compelling upgrade because they transform the AI from a generic chatbot into a context-aware collaborator. Without file reading, the AI can only reason about code pasted into the chat. With file reading, it can explore, cross-reference, and understand.

### The Capability Tiers

```
Tier 0: Chat only (paste code into conversation)
Tier 1: Read-only (file read, grep, glob, git) ← THE HYPOTHESIS
Tier 2: Read + execute (run tests, build, check)
Tier 3: Read + write (edit files, create files)
Tier 4: Full agent (read + write + execute + iterate)
```

Tier 1 is where the jump happens. The AI goes from "I can only see what you show me" to "I can explore your codebase myself." This is the difference between a chatbot and a coding tool.

### What Read-Only Gets You

1. **Codebase awareness** — The AI knows what exists, where things are, how they connect
2. **Contextual answers** — Answers grounded in actual code, not hallucinated guesses
3. **Review capability** — Proven valuable by CodeRabbit et al. (46% bug detection)
4. **Onboarding acceleration** — Weeks to days for new developers
5. **Architecture understanding** — Module boundaries, dependency flows, patterns
6. **Zero risk** — Cannot break anything, cannot introduce bugs, cannot corrupt state

### What Read-Only Costs You

1. **Implementation friction** — The AI spots the problem but the human types the fix
2. **Verification gap** — Cannot run tests to confirm suggestions work
3. **Generation inability** — Cannot scaffold, cannot create, cannot automate repetitive edits
4. **Frustration accumulation** — The better the AI understands the code, the more frustrating it is that it can't touch it

### For bpmux

Read-only tools are the natural starting point for a bot participant in a group chat. The bot can:
- Read the codebase being discussed
- Answer questions about architecture and implementation
- Review proposed changes
- Find relevant code when a topic comes up
- Trace through call chains to explain behavior

It cannot (and perhaps should not, initially) modify code. The group chat context means multiple humans and AIs are collaborating. A read-only AI that understands the code and contributes to discussion is already a meaningful participant. Write capabilities can be added as trust is established.

This aligns with the "architect before builder" principle: prove understanding before granting agency.
