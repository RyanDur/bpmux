# bpmux Agent Integration Research Report

## Current bpmux Integration Surface

bpmux exposes two integration paths:
1. **Streamable HTTP MCP endpoint** at `http://127.0.0.1:4117/mcp` (via `rmcp::StreamableHttpService`)
2. **Stdio bridge** -- a subprocess that connects to bpmux via WebSocket, used by Claude Code today

The Streamable HTTP endpoint is the simplest integration path. Any agent that can connect to an HTTP MCP server can use bpmux tools (`send_message`, `read_messages`) directly.

---

## Agent-by-Agent Assessment

### 1. GitHub Copilot (VS Code, JetBrains, CLI)

| Dimension | Status |
|-----------|--------|
| MCP support | Full MCP support in agent mode across VS Code, JetBrains, CLI |
| HTTP transport | Streamable HTTP and SSE both supported natively |
| Hook/plugin system | VS Code extension API, Copilot Extensions |
| Multi-agent/team | Copilot coding agent (cloud), SDK for agent orchestration |
| User base | Largest -- tens of millions of developers |

**Rating: Works now.** Configure in `.vscode/mcp.json`:
```json
{ "servers": { "bpmux": { "type": "http", "url": "http://127.0.0.1:4117/mcp" } } }
```

### 2. Cursor

| Dimension | Status |
|-----------|--------|
| MCP support | Full MCP support -- stdio, SSE, Streamable HTTP |
| HTTP transport | Streamable HTTP natively supported (tries POST first) |
| Hook/plugin system | Rules files, MCP servers as extension points |
| Multi-agent/team | Background agents (Cursor Agent), no native multi-agent comms |
| User base | Very large -- one of the most popular AI-native editors |

**Rating: Works now.** Configure in `.cursor/mcp.json`:
```json
{ "mcpServers": { "bpmux": { "type": "streamableHttp", "url": "http://127.0.0.1:4117/mcp" } } }
```
Note: Cursor has a [known bug](https://forum.cursor.com/t/cursor-fails-to-fall-back-from-streamable-http-to-sse-transport-for-remote-mcp-servers/154390) with SSE fallback, but since bpmux uses Streamable HTTP natively this should not matter.

### 3. Windsurf (formerly Codeium)

| Dimension | Status |
|-----------|--------|
| MCP support | Full MCP in Cascade -- stdio and Streamable HTTP |
| HTTP transport | Streamable HTTP supported natively (replaced SSE) |
| Hook/plugin system | MCP Marketplace, enterprise whitelisting |
| Multi-agent/team | Cascade sessions are single-agent, no native multi-agent |
| User base | Large -- significant IDE market share |

**Rating: Works now.** Configure via Windsurf Settings > Cascade > MCP Servers, adding an HTTP server at `http://127.0.0.1:4117/mcp`.

### 4. Aider

| Dimension | Status |
|-----------|--------|
| MCP support | No native MCP client support |
| HTTP transport | N/A -- Aider cannot connect to MCP servers natively |
| Hook/plugin system | No formal plugin system; extensible via command-line flags |
| Multi-agent/team | None |
| User base | Moderate -- popular among terminal-focused developers |

**Rating: Hard.** Aider has no native MCP client. The community has built workarounds (`mcpm-aider`, MCP-Bridge as OpenAI-compatible proxy), but none are production-ready. The [GitHub issue #3314](https://github.com/Aider-AI/aider/issues/3314) requesting native MCP client support is open. bpmux could expose a REST API (it already has `POST /api/messages` and `GET /api/messages`) that Aider could use via shell commands, but that is clunky.

### 5. Continue

| Dimension | Status |
|-----------|--------|
| MCP support | Full MCP support in agent mode -- stdio and remote HTTP |
| HTTP transport | Supports remote server connections through HTTP-based transports |
| Hook/plugin system | Open source, hub configs, mcpServers directory |
| Multi-agent/team | None natively |
| User base | Moderate -- popular open source option for VS Code and JetBrains |

**Rating: Works now.** Continue can consume JSON MCP config files from other tools directly. Drop a config in `.continue/mcpServers/` or add to hub config. HTTP MCP servers work in agent mode.

### 6. OpenCode

| Dimension | Status |
|-----------|--------|
| MCP support | MCP client built in -- stdio and remote (SSE only) |
| HTTP transport | Remote type only supports SSE, NOT Streamable HTTP |
| Hook/plugin system | None beyond MCP |
| Multi-agent/team | None |
| User base | Very large open source community (120k+ GitHub stars) |

**Rating: Easy.** OpenCode's `type: "remote"` only supports SSE transport today, and bpmux uses Streamable HTTP. There is an [open feature request](https://github.com/anomalyco/opencode/issues/8058) for Streamable HTTP support. Current workaround: use `mcp-remote` as a local stdio proxy:
```json
{ "mcpServers": { "bpmux": { "command": "npx", "args": ["-y", "mcp-remote", "http://127.0.0.1:4117/mcp"] } } }
```

### 7. Gemini Code Assist / Gemini CLI

| Dimension | Status |
|-----------|--------|
| MCP support | Full MCP -- stdio, SSE, and Streamable HTTP |
| HTTP transport | Natively supported via `httpUrl` config field |
| Hook/plugin system | Gemini CLI extensions, Google Cloud integration |
| Multi-agent/team | None natively |
| User base | Large and growing -- Google's primary coding assistant |

**Rating: Works now.** Configure with `httpUrl` in Gemini CLI or Android Studio / IDE settings pointing to `http://127.0.0.1:4117/mcp`.

### 8. Zed

| Dimension | Status |
|-----------|--------|
| MCP support | Full MCP for agent tools, stdio native, URL-based remote added recently |
| HTTP transport | Recently added `url` config for remote MCP; previously required `mcp-remote` bridge |
| Hook/plugin system | MCP extensions, Agent Client Protocol (ACP) for external agents |
| Multi-agent/team | Multiplayer editing (native), ACP connects external agents (Claude Code, Gemini CLI, Codex) |
| User base | Moderate and growing -- strong among Rust/systems developers |

**Rating: Easy.** Zed recently added URL-based remote MCP configuration:
```json
{ "context_servers": { "bpmux": { "url": "http://127.0.0.1:4117/mcp" } } }
```
If that does not work on the current Zed version, the `mcp-remote` bridge is a reliable fallback. The ACP support is also interesting -- bpmux could potentially act as a coordination layer for agents connected through Zed's ACP.

### 9. Cline

| Dimension | Status |
|-----------|--------|
| MCP support | Full MCP -- stdio, SSE, and Streamable HTTP |
| HTTP transport | Streamable HTTP natively supported and recommended |
| Hook/plugin system | MCP marketplace, auto-approve settings |
| Multi-agent/team | None |
| User base | Large -- one of the most popular VS Code AI extensions |

**Rating: Works now.** Configure in Cline's MCP settings:
```json
{ "bpmux": { "type": "streamableHttp", "url": "http://127.0.0.1:4117/mcp" } }
```
Note: Cline uses camelCase `streamableHttp` for the type field (vs kebab-case in other tools).

### 10. Amazon Q Developer

| Dimension | Status |
|-----------|--------|
| MCP support | Full MCP -- local (stdio) and remote (HTTP) |
| HTTP transport | Remote MCP servers over HTTP supported in CLI and IDE |
| Hook/plugin system | AWS integration, custom MCP servers |
| Multi-agent/team | None |
| User base | Large -- AWS's developer ecosystem |

**Rating: Works now.** Amazon Q Developer CLI and IDE extensions support remote MCP servers over HTTP with optional OAuth authentication. Point it at `http://127.0.0.1:4117/mcp`.

---

## Summary Matrix

| Agent | MCP Client | HTTP Transport | Rating | Notes |
|-------|-----------|---------------|--------|-------|
| GitHub Copilot | Yes | Streamable HTTP | **Works now** | Largest user base, immediate integration |
| Cursor | Yes | Streamable HTTP | **Works now** | Very popular, full support |
| Windsurf | Yes | Streamable HTTP | **Works now** | Full support via Cascade |
| Cline | Yes | Streamable HTTP | **Works now** | Popular VS Code extension |
| Continue | Yes | HTTP remote | **Works now** | Open source, cross-IDE |
| Gemini CLI | Yes | Streamable HTTP | **Works now** | Google ecosystem |
| Amazon Q | Yes | HTTP remote | **Works now** | AWS ecosystem |
| Zed | Yes | URL-based (recent) | **Easy** | May need mcp-remote bridge on older versions |
| OpenCode | Partial | SSE only | **Easy** | Needs mcp-remote bridge until Streamable HTTP lands |
| Aider | No | N/A | **Hard** | No native MCP client, community workarounds only |

## Key Takeaway

bpmux's choice of Streamable HTTP MCP transport turns out to be exceptionally well-positioned. **7 out of 10 agents can connect directly today with zero adapter code.** Two more need only a thin `mcp-remote` bridge (a single npx command). Only Aider lacks a clear path.

The highest-value integrations to pursue first, based on user base and ease:
1. **GitHub Copilot** -- largest user base, works immediately
2. **Cursor** -- dominant AI-native editor, works immediately
3. **Cline** -- very popular, works immediately, and its users tend to be power users who would value multi-agent coordination

Sources:
- [GitHub Copilot MCP Docs](https://docs.github.com/en/copilot/tutorials/enhance-agent-mode-with-mcp)
- [Cursor MCP Setup Guide](https://dev.to/serenitiesai/how-to-set-up-mcp-servers-in-cursor-ide-complete-guide-2026-5gdl)
- [Cursor Streamable HTTP Bug Report](https://forum.cursor.com/t/cursor-fails-to-fall-back-from-streamable-http-to-sse-transport-for-remote-mcp-servers/154390)
- [Windsurf Cascade MCP Integration](https://docs.windsurf.com/windsurf/cascade/mcp)
- [Aider MCP Support Issue #3314](https://github.com/Aider-AI/aider/issues/3314)
- [Continue MCP Docs](https://docs.continue.dev/customize/deep-dives/mcp)
- [OpenCode MCP Servers](https://opencode.ai/docs/mcp-servers/)
- [OpenCode Streamable HTTP Feature Request](https://github.com/anomalyco/opencode/issues/8058)
- [Gemini CLI MCP Docs](https://geminicli.com/docs/tools/mcp-server/)
- [Zed MCP Docs](https://zed.dev/docs/ai/mcp)
- [Zed Agent Client Protocol](https://zed.dev/acp)
- [Cline Remote Server Docs](https://docs.cline.bot/mcp/connecting-to-a-remote-server)
- [Cline HTTP Type Issue](https://github.com/cline/cline/issues/7108)
- [Amazon Q Developer MCP Docs](https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/qdev-mcp.html)
