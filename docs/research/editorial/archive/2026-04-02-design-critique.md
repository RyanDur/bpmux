# Design Critique: AI Participant Proposal

Date: 2026-04-02
Role: Editorial session (design critic)
Input: `2026-04-02-ai-participant-design.md`, `2026-04-02-ai-participant-inspirations.md`

---

## 1. Is This the Right Approach?

The proposal concludes "direct API calls" with high confidence. But there is an alternative the research touched and then dismissed too quickly: **the AI participant does not need to live inside bpmux at all.**

Consider: bpmux is a communication layer. The proposal's own Finding 7 says bpmux should NOT build coding tools because it is the communication layer. But now the proposal wants bpmux to own the conversation loop, manage context windows, handle token counting, implement streaming SSE parsing, abstract across providers, and manage system prompts. That is not a communication layer. That is an LLM application framework embedded inside a chat tool.

The alternative not seriously considered: **bpmux stays a pure message transport, and the AI participant is a separate process that connects as a peer.** This is the IRC bot model the inspirations document admires. The bot is not part of the IRC server -- it is a client. bpmux would expose a clean programmatic interface (not MCP's request/response, but a proper bidirectional message stream -- a Unix socket, a local WebSocket, or even just stdin/stdout of a child process). An external AI participant process connects, receives messages push-style, and sends responses back. bpmux does not know or care that the participant is an AI.

This preserves protocol symmetry (the inspirations document's top recommendation). It keeps bpmux simple. It lets the AI participant be written in any language, use any SDK, and evolve independently. And it means bpmux does not take on the maintenance burden of LLM client code that will break every time Anthropic or OpenAI changes their API.

The proposal rejected external processes because MCP is request/response. But MCP is not the only option for inter-process communication. The rejection conflates "external process" with "MCP."

---

## 2. What Will We Regret?

**The provider abstraction will become load-bearing infrastructure.** The proposal says the `AiProvider` trait is minimal and clean. It always starts that way. Within six months, the trait will need: tool calling (different shapes per provider), image/multimodal input, extended thinking, prompt caching (Anthropic-specific), structured output, function calling schemas, rate limit handling, retry logic, and cost tracking per provider. Each provider will accumulate special cases. The "200-300 lines" OpenAI-compat implementation will be 1500 lines with edge cases for the dozen providers that are "almost" compatible but not quite.

This is not speculation. This is the history of every provider abstraction layer ever built. The `llm` crate the proposal mentions has hundreds of issues for exactly these provider-specific behaviors.

**The system prompt is now a product decision baked into the binary.** The proposal says the system prompt is bpmux's responsibility. This means bpmux is making opinionated decisions about how the AI behaves in conversations. What happens when one user wants a terse code reviewer and another wants a verbose explainer? What happens when the system prompt needs to change for different project types? This is configuration that will balloon in complexity, and it is a fundamentally different kind of configuration than "which port to listen on."

**The two-path problem (bpmux-managed AI vs. MCP client) will not stay clean.** The proposal acknowledges this risk and suggests framing them as different use cases. In practice, users will not understand the distinction. They will ask: "Why does the built-in AI see messages instantly but my Claude Code connection has to poll? Can I make the built-in AI use tools like Claude Code does? Can I have two built-in AIs?" Every one of these questions pulls bpmux deeper into being an AI orchestration platform rather than a chat tool.

---

## 3. Is the Trigger Model Right?

The proposal defaults to `@ai` mention-based triggering. The inspirations document spends thousands of words on turn-taking theory, behavior trees, and the navigator role. Then the proposal throws all of that away and lands on "type `@ai` to invoke."

This is not pair programming. When you pair with a human, you do not prefix every remark with their name. You talk. They listen. They interject when they have something useful to say. The `@ai` model is not a participant -- it is a command invocation with a chat-shaped UI. It is `/ask` with extra steps.

The Koala research found 72% preferred reactive mode. But Koala was studying a general-purpose Slack bot in group brainstorming sessions, not a pair programming partner. Pair programming has different dynamics: the navigator is expected to speak up, not wait to be called on. The research the proposal cites does not actually validate the trigger model for the use case bpmux targets.

The deeper problem: the `@ai` trigger creates an interaction pattern where the human formulates a complete question, waits for a complete answer, then resumes work. This is synchronous question-answering. It is exactly what Claude Code already does, just rendered in a chat window instead of a terminal. It does not capture the value of having a participant who is following along and can interject at the right moment.

If the trigger model is "you must explicitly invoke the AI every time," then the AI is not a participant. It is a tool with a participant costume.

---

## 4. Is Tool Execution the Right Scope to Defer?

The proposal explicitly defers tool calling. The AI can chat but cannot read files, edit code, or run commands. The proposal says bpmux is the communication layer and participants bring their own tools.

But a coding partner who cannot see the code is not a coding partner. It is a conversation partner who happens to be in a programming context. Without file access, the AI cannot:

- Review the code someone just wrote
- Suggest a specific fix with line numbers
- Verify whether a proposed approach matches the existing codebase
- Run the test suite to check its own suggestion

The proposal's own inspirations document recommends the navigator role from pair programming. A navigator who cannot see the screen is not a navigator. They are a backseat driver operating on hearsay.

The counterargument is that users will paste code into chat. This is the 2024 pattern: copy code, paste into ChatGPT, get answer, paste back. bpmux adds a chat UI around this workflow but does not fundamentally change it. The question is whether that is enough of an improvement to justify building and maintaining LLM client infrastructure.

---

## 5. What About the User Who Already Has Claude Code?

This is the existential question the proposal does not adequately address. The target user is a developer doing pair programming. That developer almost certainly already has Claude Code, Cursor, Copilot, or similar. These tools already:

- See the code
- Edit files
- Run commands
- Stream responses
- Manage context windows
- Handle provider abstraction

What does bpmux add? The proposal's answer is "group chat" -- multiple participants, human and AI, in a shared conversation. But the value of that group chat depends on the AI participant being useful, and the AI participant as proposed (no tools, invoked by `@ai`, sliding window of 50 chat messages) is strictly less capable than the Claude Code session the developer already has open in the next terminal tab.

The real value proposition would be: the AI participant in bpmux sees the conversation between two humans and has context that Claude Code in terminal A does not -- the reasoning, the disagreements, the decisions. But the proposal's v1 cannot deliver this because the AI has no access to the codebase, only to chat messages. The chat messages are the least valuable context. The code is the most valuable context.

Put differently: if I am pairing with a colleague and we add an AI to our chat, I want the AI to understand both our conversation AND our code. A participant that only sees chat is providing advice without looking at the patient.

---

## 6. The Multi-Provider Story -- Is It Real?

The proposal invests significant architectural attention in provider abstraction: a trait, per-provider implementations, configuration for different endpoints. The justification is that developers want provider choice.

But do they? In practice, the developers who would use bpmux are likely in one of two camps:

**Camp A: Uses Claude (or GPT, or one specific model).** They have an API key for one provider. They do not want to configure bpmux for a different provider on each side of a pairing session. They want it to work with their provider.

**Camp B: Wants to use local models (Ollama).** This is a real use case for privacy-sensitive teams or offline development. But local models are dramatically worse at coding tasks than frontier models. The AI participant's usefulness depends on model quality. A local llama model as a pair programming navigator is going to produce significantly worse suggestions than Claude or GPT-4. The multi-provider abstraction may enable a use case that does not actually work well.

The provider abstraction is not free. It adds a layer of indirection to every API call, makes debugging harder (which provider is this error from?), and creates pressure to support the union of all provider features rather than the intersection. For v1, hardcoding Anthropic and adding the abstraction later when a second provider is actually needed would be simpler and would not foreclose the option.

YAGNI applies here. The proposal builds the abstraction before validating that anyone needs it.

---

## 7. What Is the Simplest Thing That Could Prove This Works?

The proposal's "minimal first step" is 500-800 lines of new Rust code across 3-4 files: a provider trait, an Anthropic implementation, a participant orchestrator, and wiring into lib.rs. That is not an experiment. That is a feature.

The simplest experiment that tests the core hypothesis ("an AI following a conversation between programmers and interjecting at the right moments is valuable") does not require any of this infrastructure. Here are cheaper ways to find out:

**Experiment A: The Wizard of Oz.** Two developers pair using bpmux as it exists today. A third person (or the developer themselves in another terminal) uses Claude Code with a prompt like "You are observing a pair programming conversation. Here is the latest exchange: [paste]. What would you say if you were a participant?" Copy the response into bpmux manually. Do this for a few sessions. Was the AI's input useful? Did the developers want it to speak up more or less? What kind of input was valuable?

This costs zero lines of code and tests the hardest hypothesis: is AI-as-participant actually valuable in pair programming?

**Experiment B: The pipe.** A shell script that tails the JSONL log, detects `@ai` messages, assembles recent context, calls the Anthropic API with `curl`, and appends the response to the log. The TUI already renders log entries. This is 50 lines of bash, not 500 lines of Rust. It will be ugly. It will not stream. It will prove whether the interaction pattern works before investing in production infrastructure.

**Experiment C: The Claude Code MCP path that already exists.** The proposal notes that MCP is already implemented. Claude Code can already connect as an MCP client. Instead of building a new AI participation mechanism, invest in making the existing MCP path less clunky. Can the MCP server push notifications? Can it use Server-Sent Events to notify the client of new messages? This tests multi-path AI participation without building a second system.

The proposal jumps from "we researched this thoroughly" to "here is the production architecture." The missing step is a cheap, disposable experiment that validates the core hypothesis before committing to 500+ lines of infrastructure code.

---

## Summary of Concerns

| Concern | Severity | Core Question |
|---------|----------|---------------|
| bpmux is becoming an LLM framework, not a chat tool | High | Should the AI participant live inside bpmux or connect as a peer? |
| Provider abstraction is premature | Medium | Does anyone need multi-provider before single-provider is validated? |
| `@ai` trigger is command invocation, not participation | High | If you must invoke it explicitly, how is this different from Claude Code? |
| No tool access makes the AI a weak coding partner | High | Can an AI that cannot see code be useful for pair programming? |
| Competing with Claude Code on its home turf | High | What does bpmux add that the user's existing AI tool does not? |
| No cheap experiment before production architecture | High | Can we test the hypothesis without building the infrastructure? |
| Two-path AI participation will confuse users | Medium | Will MCP clients and built-in AI coexist cleanly? |

The proposal is well-researched and architecturally sound in isolation. The concern is not whether it can be built, but whether it should be built this way, at this scope, before the core hypothesis is validated.
