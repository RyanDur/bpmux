# Agent Integration Research

*Designer research — 2026-04-10. Directed by Ryan.*

## Key Finding: MCP is the universal connector

Most coding agents already support MCP as a client. Connecting to bpmux is config, not code.

## Agent Compatibility

| Agent | MCP Support | Push Support | Integration | Notes |
|---|---|---|---|---|
| Claude Code | Full | Yes (channels) | Native | Current integration |
| Codex (OpenAI) | Full | No (SDK bridge needed) | 1-2 weeks | SDK enables multi-turn |
| OpenCode | Full | No | 1 day (MCP only) | No programmatic API |
| Goose (Block) | Full, MCP-first | No | 1 day | Rust, Apache 2.0, best non-Claude candidate |
| Cline | Full | Partial (headless) | 1 week | Headless mode |
| Aider | Community wrappers | No | 1 week | Python, simple |
| MLX (local) | Build yourself | Build yourself | 4-8 weeks | Use Goose + Ollama instead |

## The Push Gap

Only Claude Code has push-based channels. Every other agent is pull-only — must explicitly call read_messages. This means nudges, context-triggered recall, and pile-on detection don't work proactively for non-Claude agents.

## Fastest Path for Tomorrow (MLX on new machine)

Don't build a custom agent. Use **Goose** (Rust, MCP-first, Apache 2.0) configured with a local Ollama/MLX model endpoint. Connect Goose to bpmux via MCP config. Much less effort than building from scratch.

## nono Sandboxing

Ryan flagged github.com/always-further/nono — kernel-enforced sandbox for AI agents. Uses Landlock (Linux) and Seatbelt (macOS). Relevant to bpmux: when P2P networking opens, agents gain network access. nono restricts what agents can reach at the OS level. Created by the Sigstore author (Luke Hinds). Worth adopting when we open network connectivity.

## What bpmux Already Provides

- WebSocket endpoint (/ws) — bidirectional, push-capable
- MCP server with send_message, read_messages, search_log, list_lessons, save_lesson
- JSONL log for history

## What Would Help

- Example configs for each agent (Codex config.toml, OpenCode opencode.jsonc, Goose MCP config)
- A raw text WebSocket endpoint for simpler bridges
- Documentation for non-Claude integration
