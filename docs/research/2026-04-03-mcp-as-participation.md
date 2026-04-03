# MCP as Participation: Can Claude Code Be a Real bpmux Participant Without a Separate Bot?

Research question: Can we make Claude Code participate in bpmux conversations just by being smart about MCP tool descriptions, hooks, CLAUDE.md instructions, and protocol capabilities -- without building a separate bot process?

## TL;DR

**Claude Code Channels is the answer.** Shipped in v2.1.80 (March 2026), channels let an MCP server push messages directly into a running Claude Code session. bpmux could declare `claude/channel` capability and push new chat messages as channel events. Claude Code would see them and respond without the human switching contexts. Everything else researched here is a partial solution at best.

---

## 1. Tool Descriptions as Behavior Instructions

**Question:** If read_messages says "Call this at the start of every response to check for new messages" -- does Claude Code follow that?

**Finding: Partially, unreliably.**

An empirical study on MCP tool descriptions (arxiv.org/html/2602.14878) found:
- Augmented descriptions improved task success rates by a median of 5.85 percentage points
- But caused **regressions in 16.67% of cases**
- Richer descriptions increased execution steps by 67.46%, consuming more tokens
- "No single combination of MCP tool description components consistently yielded improved performance across all domains and models"

Tool descriptions influence tool *selection* and *parameterization* well. They are weaker at influencing *when* to call a tool proactively. The description "call this before every response" is a behavioral instruction, not a usage specification -- models treat these differently.

Additionally, Claude Code uses **tool search** by default, which defers loading tool definitions until Claude determines it needs them. A behavioral instruction in a tool description that never gets loaded has zero effect.

**Verdict: Fragile. Would work sometimes, fail unpredictably. Not a foundation to build on.**

## 2. MCP Notifications / Server-Sent Events

**Question:** Can the MCP server push a notification to Claude Code saying "new messages available"?

**Finding: The spec supports it. Claude Code's handling is limited.**

The MCP spec defines several notification types:
- `notifications/resources/list_changed` -- server's resource list changed
- `notifications/resources/updated` -- a specific subscribed resource changed
- `notifications/tools/list_changed` -- server's tool list changed

Claude Code *does* handle `list_changed` notifications to refresh available tools/resources. But standard MCP notifications are metadata updates ("my tool list changed"), not content delivery ("here's a new message for you to read and respond to"). There's no spec mechanism for "here's content, please act on it" -- that's what **sampling** and **channels** are for.

**Verdict: MCP notifications are for metadata refresh, not content delivery. Wrong mechanism.**

## 3. MCP Sampling

**Question:** When a new message arrives, could bpmux ask Claude Code to sample/respond via the sampling API?

**Finding: The spec supports this perfectly. Claude Code does not implement it.**

The MCP sampling capability (`sampling/createMessage`) lets a server request the client to generate an LLM response. The server sends messages, the client runs inference, returns the result. The Nov 2025 spec even added tool use within sampling, enabling full agentic loops.

However: **Claude Code does not support MCP sampling as a client.** GitHub issue #1785 (103 upvotes, assigned to an Anthropic engineer) confirms this. The official response was "We're looking into this" -- still open. VS Code has implemented sampling support, setting a precedent.

If/when Claude Code adds sampling support, bpmux could:
1. Detect new message arrives in the chat
2. Send `sampling/createMessage` with the conversation history
3. Claude Code runs inference and returns a response
4. bpmux posts the response to the chat

This would be the cleanest protocol-level solution. But it doesn't exist yet.

**rmcp supports it:** The Rust MCP SDK (rmcp) has sampling examples and full support for the capability. So bpmux could implement the server side today and it would work the day Claude Code adds client support.

**Verdict: Best protocol-level design. Blocked by Claude Code not implementing it. Worth preparing for.**

## 4. MCP Resources with Subscriptions

**Question:** Could the message log be a resource that Claude Code subscribes to?

**Finding: The spec supports subscriptions. Claude Code's behavior on update is unclear.**

The MCP resources spec allows:
1. Server declares `resources` capability with `subscribe: true`
2. Client sends `resources/subscribe` for a specific URI
3. Server sends `notifications/resources/updated` when the resource changes
4. Client re-reads the resource

bpmux could expose `bpmux://session/{id}/messages` as a resource. Claude Code could subscribe. When a new message arrives, bpmux notifies, Claude Code re-reads.

The problem: what happens after Claude Code re-reads the resource? In Claude Desktop, resources populate context but don't trigger action. The model sees updated context but has no instruction to *respond* to it. Resource subscriptions are for **context freshness**, not **reactive behavior**.

**Verdict: Would keep Claude Code's context fresh but wouldn't trigger responses. Necessary complement, not sufficient alone.**

## 5. Claude Code Hooks

**Question:** Could hooks inject "check bpmux for new messages" into Claude Code's flow?

**Finding: Yes. The Stop hook is particularly relevant.**

Claude Code hooks are powerful and well-documented. Key capabilities:

| Hook | Use for bpmux |
|------|--------------|
| **Stop** | When Claude finishes responding, a Stop hook could check bpmux for new messages and inject them as `additionalContext`, blocking the stop and forcing continuation |
| **SessionStart** | Inject "you are in a bpmux session" context at startup |
| **PostToolUse** | After any tool call, check for new messages |
| **UserPromptSubmit** | Prepend latest bpmux messages to every user prompt |

The **Stop hook** is the most interesting. It:
- Fires when Claude finishes responding
- Can return `decision: "block"` with a `reason` to prevent stopping
- Can inject `additionalContext` that Claude sees
- Has a `stop_hook_active` guard to prevent infinite loops

A Stop hook could: poll bpmux for new messages -> if found, block the stop and inject them -> Claude sees the messages and responds -> the cycle repeats until no new messages.

The guard rail: `stop_hook_active` is true when Claude is already continuing from a stop hook, preventing infinite loops. The hook script would need to check this and allow the stop when no new messages exist.

**Configuration would go in `.claude/settings.json`:**
```json
{
  "hooks": {
    "Stop": [{
      "hooks": [{
        "type": "command",
        "command": "/path/to/bpmux-check-messages.sh"
      }]
    }],
    "SessionStart": [{
      "matcher": "startup",
      "hooks": [{
        "type": "command",
        "command": "/path/to/bpmux-session-context.sh"
      }]
    }]
  }
}
```

**Verdict: Works today. Polling-based, not push. Adds latency (only checks at stop boundaries). But functional and reliable.**

## 6. CLAUDE.md / System Prompt Injection

**Question:** Could CLAUDE.md reliably change Claude Code's behavior to check messages before each response?

**Finding: Somewhat reliable for tool selection guidance, unreliable for behavioral loops.**

CLAUDE.md content is injected as high-priority context. Claude Code's system prompt includes `<system-reminder>` tags with CLAUDE.md contents. Instructions there do influence behavior.

However:
- CLAUDE.md is loaded once at session start (and on compaction). It's static context, not a trigger mechanism.
- Instructions like "before each response, call read_messages" work best when the user's request is related to bpmux. When the user asks "fix this function," Claude has no strong reason to check chat messages first.
- The instruction competes with Claude's core system prompt and the user's actual request. Over a long session, adherence degrades.
- With tool search enabled (default), the read_messages tool definition may not even be loaded unless Claude determines it's relevant.

CLAUDE.md is best used for: declaring the bpmux context ("you are participating in a bpmux session"), defining conventions ("sign messages as Claude"), and providing the server URL. Not for behavioral loops.

**Verdict: Good for context-setting. Bad for behavioral enforcement. Complement, not solution.**

## 7. Claude Code Channels (The Real Answer)

**Question:** Is there a mechanism purpose-built for this?

**Finding: Yes. Claude Code Channels shipped March 2026.**

Channels are MCP servers that declare `claude/channel` capability and can push events into a running Claude Code session. This is exactly the pattern bpmux needs.

**How it works:**

1. MCP server declares the capability:
   ```typescript
   capabilities: { experimental: { 'claude/channel': {} } }
   ```

2. When an event arrives (new bpmux message), the server pushes a notification:
   ```typescript
   await mcp.notification({
     method: 'notifications/claude/channel',
     params: {
       content: messageBody,
       meta: { path: '/messages', method: 'POST' }
     }
   })
   ```

3. The message arrives in Claude's context as a `<channel>` tag. Claude reads it and can act on it -- no human intervention required.

4. Claude Code is started with `--channels` to enable the server:
   ```bash
   claude --channels bpmux
   ```

**Real-world validation:** The `claude-peers-mcp` project (github.com/louislva/claude-peers-mcp) already does exactly this pattern -- multiple Claude Code instances discovering each other and exchanging messages in real-time via channels. It uses a broker daemon on localhost with SQLite, plus channel push for instant delivery.

**Security model:** Two layers. First, sender allowlists (only approved senders can push). Second, the `--channels` flag (server must be explicitly enabled per session). This aligns with bpmux's security model.

**Requirements:**
- Claude Code v2.1.80+ (current)
- Bun runtime (channels use Bun currently -- but the protocol is just MCP notifications)
- claude.ai authentication (channels require Pro/Max, not API keys)
- Research preview status (may change)

**What bpmux would need to do:**
1. Declare `claude/channel` in the MCP server capabilities
2. When a new message arrives in the bpmux session from another participant, push it as a channel notification
3. Expose a `reply` tool so Claude can send responses back
4. The existing `send_message` and `read_messages` tools continue working for manual use

**Verdict: Purpose-built for this use case. Real-world validated. The answer.**

## 8. Real-World Examples

### claude-peers-mcp
Multiple Claude Code instances communicating in real-time. Uses channels for push delivery, SQLite broker for message storage. Exactly the pattern bpmux needs.

### Telegram/Discord channels
Official Anthropic plugins that bridge external chat platforms into Claude Code sessions. Messages from Telegram users push into Claude's context. Claude responds via exposed `reply` tools. Demonstrates the pattern works for chat participation.

### Chrome DevTools MCP
Example of behavioral instructions in tool descriptions working. The DevTools MCP uses detailed tool descriptions to guide Claude's debugging workflow. However, this is tool selection guidance, not proactive behavior.

## 9. The Honest Assessment

### What works today (no new features needed)

**Hooks + CLAUDE.md combination.** A Stop hook polls for new messages at each turn boundary. CLAUDE.md provides session context. This creates a pull-based participation model: Claude checks for messages whenever it finishes a task. Latency is bounded by turn duration (could be seconds or minutes).

**Channels (research preview).** If bpmux declares `claude/channel`, it can push messages in real-time. Claude sees them immediately. This is the push model. It works today but is in research preview and requires Bun runtime.

### What doesn't work yet

**Sampling.** The cleanest server-initiated interaction model. Not implemented in Claude Code. Worth preparing for.

### The user experience, honestly

**With hooks only:** Human types in bpmux terminal. Eventually Claude Code finishes its current task, the Stop hook fires, finds the message, injects it. Claude responds. Claude sends back via send_message. The bpmux terminal shows the response. It works, but there's a lag and it only happens at turn boundaries.

**With channels:** Human types in bpmux terminal. bpmux pushes the message as a channel event. Claude sees it immediately, responds, sends back. Near-real-time. This is genuinely useful for pair programming -- the human can talk to Claude through bpmux while Claude is working in the codebase.

**The context-switch tax:** Even with channels, Claude Code is a separate terminal. The human has two views: bpmux (chat) and Claude Code (coding). But this is actually natural for pair programming -- one screen for conversation, one for code. The channel model means Claude participates in both simultaneously.

### Recommendation

1. **Implement channels support in bpmux's MCP server.** Declare `claude/channel`, push new messages as channel notifications. This is the primary mechanism.

2. **Keep the existing tools as fallback.** `read_messages` and `send_message` work without channels for manual use.

3. **Add a Stop hook script to the project** as a fallback for environments where channels aren't available.

4. **Prepare the sampling server-side** in rmcp. When Claude Code adds sampling support, bpmux can use it for more structured interactions (e.g., "summarize the last 5 minutes of discussion").

5. **Don't rely on tool descriptions or CLAUDE.md for behavioral loops.** Use them for context and conventions only.

## Sources

- MCP Sampling Spec: https://modelcontextprotocol.io/specification/2025-11-25/client/sampling
- MCP Resources Spec: https://modelcontextprotocol.io/specification/2025-11-25/server/resources
- Claude Code Hooks: https://code.claude.com/docs/en/hooks
- Claude Code MCP: https://code.claude.com/docs/en/mcp
- Claude Code Sampling Issue: https://github.com/anthropics/claude-code/issues/1785
- MCP Tool Description Smells: https://arxiv.org/html/2602.14878
- claude-peers-mcp: https://github.com/louislva/claude-peers-mcp
- rmcp Rust SDK: https://github.com/modelcontextprotocol/rust-sdk
- MCP Notifications Discussion: https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/1192
- Claude Code Channels Guide: https://claudefa.st/blog/guide/development/claude-code-channels
