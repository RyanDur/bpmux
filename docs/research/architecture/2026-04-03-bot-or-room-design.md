# Design Tension: Should bpmux Ship a Built-in Bot?

The original principle: "bpmux is the room, not the AI." The emerging problem: an empty room where nobody talks back is not compelling. This document researches the design space to inform the decision.

## The Three Options

**A. Built-in bot** — `bpmux bot` subcommand. Single binary, works out of the box.
**B. Pure communication layer** — bpmux is only the room. External tools (Claude Code, OpenCode, custom bots) provide the AI.
**C. Minimal replaceable bot** — Ship a basic bot that demonstrates the pattern but is designed to be swapped out.

---

## 1. Precedent from Chat Platforms

### IRC: No Built-in Bot, Rich Ecosystem

IRC never shipped a bot. Eggdrop (1993) and Infobot pioneered the pattern, and a vast ecosystem followed. The result: powerful for experts, hostile to newcomers. IRC required significant technical knowledge to get bots running, and non-power users struggled with setup. The ecosystem was open and decentralized but never reached mainstream adoption. IRC's user base steadily declined as Slack and Discord grew.

### Slack: Slackbot as Onboarding Device

Slack's Slackbot exists for one reason: when users drop into a new chat product, they have nobody to talk to. Logging into a social product with no activity is a deal-killer (the "empty Facebook" problem). Slackbot solves this by being the first entity that responds.

Critically, Slackbot does not try to be sophisticated. It declares itself "just a bot" upfront, which disarms skepticism and high expectations. It covers basics one step at a time without requiring tutorials or demos. The design is a collection of triggered messages, not an advanced AI. Slackbot's purpose is onboarding and demonstrating the platform's capabilities to new users.

Slack then built a formal bot ecosystem on top: slash commands, the App Directory, a review process. Third-party bots do the heavy lifting. Slackbot showed the room was alive; the ecosystem provided real value.

### Discord: Built-in AutoMod + Ecosystem Coexistence

Discord launched AutoMod in June 2022 as a built-in moderation bot. Since launch, it has blocked over 45 million unwanted messages. But Discord explicitly recommends a tiered approach:

- Small communities: AutoMod alone is sufficient
- Mid-size (1,000+): AutoMod + specialized third-party bots
- Large (10,000+): Multiple specialized bots alongside AutoMod
- Very large (100,000+): Full bot fleet with AutoMod as baseline

AutoMod serves as a foundational layer that works alongside third-party solutions. It did not replace the bot ecosystem. It gave every server a working baseline and demonstrated the pattern that bots could follow.

### Pattern

Every successful modern chat platform ships a built-in bot, but constrains it. The built-in bot serves onboarding and demonstrates platform capabilities. The serious work happens through the ecosystem. Platforms that shipped without any bot (IRC) had expert-only adoption.

---

## 2. Batteries Included vs. Bring Your Own

### The Adoption Data

VS Code holds 75.9% of the developer editor market as of 2025 (Stack Overflow survey, 49,000 developers, 177 countries). It reached #1 in 2018 at 35% and has grown every year since. The batteries-included approach (IntelliSense, built-in terminal, Git integration, debugger) combined with extensibility dominates.

Neovim and Vim combined hold a fraction of that share. Developers report that configuring Neovim is "time-consuming" and involves "spending more time configuring than coding." The bring-your-own-everything approach produces passionate users but small market share.

42% of developers use more than one IDE. This suggests even batteries-included users want options, but they need a working default first.

### The Lesson for bpmux

The data is unambiguous: batteries-included wins adoption. But VS Code did not win by being unconfigurable. It won by working immediately and being replaceable in every dimension. The built-in terminal is fine; you can still use iTerm. The built-in Git is fine; you can still use the CLI. The built-in debugger is fine; you can still use lldb.

The question is not "should bpmux have a default?" but "how replaceable should the default be?"

---

## 3. The Onboarding Problem

### Friction Kills Adoption

Research on developer tool onboarding is stark:

- Average onboarding abandonment rates sit between 60-80% across software tools
- Users who do not engage within the first 3 days have a 90% chance of churning
- Personalized onboarding paths increase completion rates by 35%
- Google reduced new-hire onboarding from 6 weeks to 3 days by automating 95% of local setup

The Atlassian 2025 State of DevEx Survey (3,500 developers, 6 countries) found that developers spend only 16% of their time coding, and that AI toolchain fragmentation (juggling multiple disconnected AI tools) is a major friction source.

### What This Means for bpmux Without a Bot

If bpmux ships without a bot, the first-time experience is:

1. Install bpmux
2. Start bpmux
3. See an empty chat
4. Realize you need an external AI tool
5. Research which tools work with bpmux
6. Install and configure that tool
7. Connect it to bpmux
8. Finally have a conversation

Steps 4-7 are where the 60-80% abandonment happens. Every additional step between "install" and "value" loses users exponentially. The empty room is the worst possible first impression because it communicates "this product does nothing."

### Slack's Insight

Slack understood this. The reason Slackbot exists is not technical sophistication but the recognition that an empty chat product is a dead chat product. The first message matters more than the architecture.

---

## 4. The Minimum Viable Bot

### What "Good Enough" Looks Like

A minimum viable bot for bpmux needs exactly:

1. **System prompt** — establishes the bot's role in the conversation
2. **Conversation history** — reads the bpmux message log so it has context
3. **LLM API call** — sends context + new message, streams the response
4. **Message posting** — writes the response back into the bpmux chat

That is: read the log, call an API, write the response. The implementation is likely under 500 lines of Rust.

### What It Does Not Need

- Tool use (file reading, code execution) — that is what Claude Code, OpenCode, and dedicated agents do
- RAG or retrieval — the conversation history IS the context
- Plugin systems — the bot itself is the plugin, replaceable by design
- Sophisticated prompt engineering — a simple system prompt ("You are a participant in a pair programming session") is sufficient to demonstrate value

### Is It Good Enough?

A simple chat bot is not as capable as Claude Code for coding tasks. But that is not the point. The point is:

- It proves the room is alive
- It demonstrates the message protocol works
- It gives new users someone to talk to immediately
- It shows what a bot integration looks like, so others can build better ones

The minimum viable bot is a reference implementation, not the product. Its job is to make the room useful on day one, not to be the best AI assistant.

---

## 5. The Tool-Agnostic Promise

### Does a Built-in Bot Undermine Neutrality?

Platform strategy research suggests the opposite: a reference implementation strengthens a tool-agnostic claim. The reasoning:

- Kubernetes ships with a default scheduler, default DNS (CoreDNS), and default networking. All are pluggable. The defaults do not undermine Kubernetes' extensibility claim — they demonstrate the extension points work.
- Kubernetes' design philosophy is explicitly "not monolithic" with "default solutions that are optional and pluggable." It provides "building blocks for building developer platforms" while preserving "user choice and flexibility."
- Platform-agnostic does not mean no platform. It means selective use: accept a default for most cases while keeping escape hatches.

### The Reference Implementation Pattern

Shipping a built-in bot that is explicitly replaceable serves the tool-agnostic promise better than shipping no bot at all, because:

1. **It proves the protocol works.** An empty room with documentation about how bots could connect is less convincing than a working bot that demonstrates the protocol.
2. **It lowers the barrier to alternative bots.** Developers building a custom bot can look at the built-in one as a reference. "Here is how to read the message log, post a message, and integrate an LLM" is more useful than "here is a protocol specification."
3. **It establishes the pattern.** The built-in bot shows: a bot is a participant in the chat, not a privileged system process. Other bots follow the same pattern.

The risk is if the built-in bot becomes so entangled that replacing it is difficult. The mitigation is clear: the bot must connect to bpmux through the exact same interface that any external bot would use. No privileged access. No special APIs.

---

## 6. What Anthropic's Own Tools Do

### Claude Code: The Complete Experience

Claude Code ships as a complete package: model + tools + UI + file system access + MCP extensibility. The design philosophy is "put as little code as possible around the model" with the thesis that "a simple, single-threaded master loop combined with disciplined tools delivers controllable autonomy."

Claude Code is not a chat UI that requires you to bring your own model. It is a working product from first launch. But it is also extensible: MCP servers, custom tools, CLAUDE.md configuration, hooks.

### The Claude Agent SDK: The Runtime as a Library

The Claude Agent SDK is the runtime that powers Claude Code, extracted as a library. It provides: built-in tools (file read, file edit, bash, web search), automatic context management, session persistence, permissions, subagent orchestration, and MCP extensibility.

The SDK's philosophy: "give agents a computer, not just a prompt." It is a comprehensive runtime environment, not a text generation wrapper. Custom tools, lifecycle hooks, and MCP servers allow extending or replacing any capability.

### MCP: The Protocol Layer

MCP (Model Context Protocol) has achieved industry-wide adoption: 10,000+ active public servers, adopted by ChatGPT, Cursor, Gemini, VS Code, and others. Server downloads grew from 100,000 to 8 million in six months. It was donated to the Linux Foundation under the Agentic AI Foundation.

MCP is relevant to bpmux because it demonstrates how Anthropic approaches the "built-in vs. extensible" question: ship a working default (Claude Code with built-in tools), define an open protocol (MCP), and let the ecosystem build on top.

### Pattern for bpmux

Anthropic's pattern is: complete experience by default, open protocol for extension, ecosystem on top. They did not ship Claude Code as "here is a terminal UI, bring your own model and tools." They shipped a working product and then made every layer replaceable.

---

## Synthesis

### The Evidence Points to Option C

Every signal from the research converges on the same answer: ship a minimal, replaceable bot.

| Signal | What It Says |
|---|---|
| Chat platform history | Every successful platform ships a default bot. IRC (no bot) lost to Slack/Discord (built-in bot). |
| Adoption data | Batteries-included tools dominate. VS Code at 75.9%. Empty-room tools are expert-only. |
| Onboarding research | 60-80% abandonment when setup is required. First 3 days determine retention. |
| Platform strategy | Reference implementations strengthen tool-agnostic claims (Kubernetes pattern). |
| Anthropic's own approach | Ship complete, make replaceable. Claude Code + MCP + Agent SDK. |

### What Option C Looks Like

1. `bpmux` starts a session. A minimal bot joins automatically.
2. The bot reads the conversation log, calls an LLM API, posts responses.
3. The bot uses the same protocol any external bot would use. No privileged access.
4. `bpmux --no-bot` disables it. `bpmux --bot=<path>` replaces it.
5. The bot's source is a clear, documented example of how to build a bpmux participant.

### What It Does NOT Look Like

- The bot is not deeply integrated into the bpmux core. It is a separate concern that happens to ship in the same binary.
- The bot does not have tool use, file access, or code execution. That is what Claude Code and dedicated agents provide.
- The bot is not the product. The room is the product. The bot is a participant that makes the room immediately useful.

### The Principle, Revised

"bpmux is the room, not the AI" remains true. But the room comes with a greeter. The greeter can be replaced, upgraded, or removed. The greeter exists because an empty room with a sign saying "bring your own conversationalist" is not a room anyone wants to enter.

---

## Open Questions

1. **API key management.** A built-in bot needs an LLM API key. Where does it come from? Environment variable? Config file? First-run wizard? Each adds friction, partially defeating the purpose.

2. **Which model?** The bot needs to call some LLM. Hardcoding Claude (via Anthropic API) is the obvious choice but creates a vendor dependency. Supporting multiple providers adds complexity. The minimum viable answer is probably: support the Anthropic API by default, accept an OpenAI-compatible endpoint as an alternative, keep the interface simple enough that swapping is trivial.

3. **Bot identity in the chat.** Is the built-in bot a named participant ("Claude")? An anonymous "Assistant"? A configurable name? This affects how users perceive the room and whether the bot feels like a first-class participant or a system feature.

4. **When to replace the built-in bot.** The documentation should make it clear: the built-in bot is for getting started. For real pair programming, connect Claude Code or your preferred AI tool. The built-in bot is training wheels, not the bicycle.

5. **Scope creep risk.** Once a bot exists in the binary, there will be pressure to add features (tool use, file reading, context expansion). The design must resist this. The built-in bot stays minimal. Feature-rich AI participation comes from external tools. The line must be bright.

---

## Confidence Assessment

**High confidence:** A built-in bot is necessary for adoption. The chat platform precedent, onboarding research, and batteries-included data all point the same direction. Shipping without a bot is choosing the IRC path.

**Medium confidence:** Option C (minimal, replaceable) is better than Option A (full-featured built-in). The reasoning is sound (tool-agnostic promise, scope creep prevention, Kubernetes pattern), but there is a real risk that "minimal" feels inadequate and users judge bpmux by the built-in bot's limitations rather than the room's potential.

**Low confidence:** The exact minimal feature set. "System prompt + history + API call" is a hypothesis. It might turn out that without at least basic tool use (reading the current file), the bot is too dumb to demonstrate pair programming value. This needs to be tested, not theorized.
