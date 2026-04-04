# Cross-Tool Integration: Research Findings

*April 4, 2026. Evaluating the cross-tool collaboration problem for bpmux against grounding theory, ChatCollab validation, A2A, MCP, and multi-agent framework research.*

---

## 1. What Does the Research Say About Cross-Tool Collaboration?

Clark & Brennan's constraint model predicts that grounding costs are determined by the communication medium's properties. When participants use different interfaces to the same shared channel, they experience different constraint profiles -- and therefore different grounding costs for the same conversation.

### The asymmetric cotemporality problem

bpmux already identified this in the Clark & Brennan deep read: human TUI users have cotemporality (real-time WebSocket push), while AI participants using `read_messages` have only reviewability (polling). The constraint table from that analysis shows bpmux's AI participant profile is identical to email.

The cross-tool problem makes this worse. Different AI tools have different integration capabilities:

- **Claude Code** can use MCP tools, hooks (Stop/UserMessage), and experimental channels for push notifications.
- **Cursor/Windsurf** support MCP tools but have different hook systems and no equivalent to Claude Code channels.
- **GitHub Copilot** added MCP support via VS Code in mid-2025, but the integration model is different -- MCP servers run as VS Code extensions, not standalone processes.
- **Gemini** supports MCP but with its own transport preferences.

Clark & Brennan's framework predicts that when one participant has cotemporality and another does not, the cotemporal participant will expect conversational-speed grounding (quick acknowledgments, relevant next turns) while the non-cotemporal participant operates on email-speed grounding (batch reading, long-form responses). The mismatch creates failed expectations: the cotemporal participant interprets silence as State 0 (never saw the message) when the non-cotemporal participant may simply not have polled yet.

### The heterogeneous constraint profile

In a bpmux session with mixed tools, each participant has a different constraint profile:

| Constraint | Human (TUI) | Claude Code (hooks + channels) | Claude Code (hooks only) | Cursor/other (MCP only) |
|---|---|---|---|---|
| Cotemporality | Yes (WebSocket) | Partial (channels experimental) | No (poll on turn boundary) | No (poll on tool call) |
| Sequentiality | Yes | Yes | Yes | Yes |
| Reviewability | Yes | Yes | Yes | Yes |
| Simultaneity | Partial | No | No | No |

Clark & Brennan never analyzed a medium where participants have different constraint profiles simultaneously. Their model assumes the medium is shared -- everyone on the telephone has cotemporality, everyone writing email lacks it. bpmux breaks that assumption. The prediction from extending their framework: grounding costs will be determined by the least-capable participant's constraints. If one participant polls every 5 minutes, the effective cotemporality of the entire session degrades to 5-minute latency for any exchange involving that participant. The shared channel is only as fast as the slowest reader.

This is the **lowest-common-denominator problem**: a heterogeneous medium converges to the constraint profile of its most limited participant, unless the protocol provides mechanisms for participants with richer capabilities to operate independently (sub-channels, direct messaging, or tiered conversations).

---

## 2. Does the ChatCollab Validation Hold When the Medium Is Heterogeneous?

ChatCollab used Slack as the shared channel. Every participant -- human and AI -- used the same interface with the same capabilities. The paper explicitly states: "It is the only way agents communicate with each other." This homogeneity is not incidental to their findings. It is structural.

### What homogeneity gave ChatCollab

1. **Uniform cotemporality.** All agents polled the Slack timeline at similar intervals (3-15 second random pauses). All participants experienced roughly the same latency. Grounding costs were symmetric.

2. **Shared typing awareness.** When an agent started composing, a typing event appeared for all participants. This mechanism only works if all participants can both produce and receive typing events through the same interface.

3. **Identical affordances.** Every participant could send messages, receive messages, and generate file artifacts through the same Slack API. No participant had capabilities others lacked.

4. **Symmetric grounding criterion.** Because all participants experienced the same medium, the "sufficient for current purposes" threshold was calibrated to the same constraint profile. Everyone operated at the same email-plus-cotemporality level.

### What bpmux's heterogeneity breaks

bpmux violates all four assumptions:

1. **Asymmetric cotemporality.** A Claude Code instance with channels gets push notifications. A Cursor instance with MCP-only gets messages when it calls `read_messages`. The grounding latency is fundamentally different.

2. **No universal typing awareness.** Even if bpmux added a typing/composing broadcast, only WebSocket-connected clients would receive it in real-time. MCP-only clients would not see it until their next poll.

3. **Different affordances.** Claude Code hooks can inject bpmux messages into the AI's context automatically. Other tools require the AI to explicitly call the `read_messages` tool. The former is passive reception; the latter is active retrieval. These are different cognitive models for the AI.

4. **Asymmetric grounding criteria.** A participant with push notifications can ground at conversational speed. A participant with polling can only ground at batch speed. The same conversation operates at two different grounding criteria simultaneously.

### Does the validation still hold?

Partially. ChatCollab validates the core architectural claim: a shared message channel with event-driven autonomy produces better collaboration outcomes than sequential pipelines. That finding transfers to bpmux regardless of interface heterogeneity. The channel model is sound.

But ChatCollab does not validate that the model works when participants have different constraint profiles. Their experimental design controlled for this variable by eliminating it. The specific findings about role differentiation, behavioral configurability, and human substitution depend on all participants experiencing the same medium. Whether a Cursor agent and a Claude Code agent exhibit the same emergent role differentiation when they have different affordances is an open empirical question.

The honest assessment: bpmux inherits ChatCollab's validation for the channel architecture but needs its own validation for the heterogeneous participant model. No existing research covers this case.

---

## 3. Does A2A Solve the Multi-Tool Problem?

### What A2A provides

The Agent2Agent Protocol (A2A), launched by Google in April 2025 and donated to the Linux Foundation in June 2025, is designed for cross-vendor agent interoperability. Its core abstractions:

- **Agent Cards** (`.well-known/agent.json`): JSON metadata documents describing an agent's identity, capabilities, skills, service endpoint, and authentication requirements. This is agent discovery -- a client agent fetches the card to see who can do what.

- **Task lifecycle**: Tasks are the fundamental unit of work, identified by unique IDs, progressing through defined states (submitted, working, input-required, completed, failed, canceled, rejected).

- **Flexible communication**: Synchronous request/response via `task/send`, streaming via `task/sendSubscribe` (SSE), and push notifications for long-running tasks.

- **Opaque agents**: A2A treats agents as black boxes. A client agent does not need to know the remote agent's implementation, framework, or model. It only needs the Agent Card.

By December 2025, the Linux Foundation launched the Agentic AI Foundation (AAIF) -- co-founded by OpenAI, Anthropic, Google, Microsoft, AWS, and Block -- as the permanent home for both A2A and MCP. IBM's Agent Communication Protocol (ACP) merged into A2A in August 2025. The protocol reached version 0.3 with gRPC support.

### Does A2A solve bpmux's problem?

No. A2A solves a different problem.

A2A is a **delegation protocol**. Agent A discovers Agent B's capabilities, sends Agent B a task, and receives results. The interaction model is client-remote: one agent asks, another agent does. This is RPC with discovery, not shared collaboration.

bpmux needs a **participation protocol**. Multiple agents observe the same shared timeline, contribute to it as peers, and build mutual understanding through conversational exchange. There is no client-remote distinction. There is no task delegation. There is group communication.

The specific gaps:

1. **A2A has no shared channel.** Communication is point-to-point between a client and a remote agent. There is no concept of a group conversation that all agents observe. A2A tasks are bilateral, not multilateral.

2. **A2A's task model does not map to conversation.** bpmux messages are contributions to a shared discourse. They are not tasks with lifecycle states. A message like "I disagree with the auth approach" is not submitted, working, or completed. It is a grounding contribution.

3. **A2A does not address grounding.** The protocol handles task completion, not mutual understanding. Whether the remote agent understood the request is determined by whether the task succeeds, not by evidence of comprehension.

4. **A2A discovery is for capabilities, not presence.** Agent Cards describe what an agent can do, not whether it is currently participating in a conversation. bpmux needs presence awareness (who is connected, who is reading), not capability discovery.

### Where A2A could complement bpmux

A2A's Agent Card concept is relevant for one specific scenario: when a bpmux session needs to discover and invite agents running on remote platforms. If a Cursor user wants to join a bpmux session, an A2A-style discovery mechanism could help bpmux find and connect to the Cursor agent's MCP endpoint. But this is discovery and connection setup, not the ongoing collaboration protocol.

The industry consensus as of early 2026 is that MCP and A2A are complementary: "MCP powers agents internally with context and tools, while A2A connects agents externally for collaboration and task execution." bpmux sits at a different layer than either. bpmux is the shared channel. MCP is how agents connect to it. A2A is how agents find each other before connecting. None of these protocols provide what bpmux itself provides: a persistent, shared, reviewable conversation timeline for group grounding.

---

## 4. Is MCP Sufficient for bpmux's Needs?

### What MCP provides today

MCP has achieved universal adoption. As of early 2026, it has crossed 97 million monthly SDK downloads and is supported by every major AI tool: Claude Code, Cursor, Windsurf, GitHub Copilot, Gemini, and dozens more. It standardizes how AI tools connect to external capabilities.

For bpmux, MCP provides:

- **Tool interface**: `send_message` and `read_messages` as MCP tools that any compliant client can call.
- **Universal compatibility**: Any MCP-capable tool can connect to bpmux's bridge server.
- **Stdio transport**: The bridge runs as a local process that Claude Code (or any MCP client) spawns and communicates with via stdin/stdout.
- **Streamable HTTP transport**: The 2025-03-26 spec added HTTP-based transport with SSE for server-to-client streaming, enabling remote MCP servers.

### What MCP lacks for bpmux

**Push notifications are not reliably available.** The MCP spec supports server-to-client notifications via SSE, but the practical reality is fragmented:

- Claude Code has experimental "channels" (`claude/channel` capability) that allow an MCP server to push messages into the conversation context. This is the push mechanism bpmux needs. But it is experimental, has no stability guarantee, and is Claude Code-specific.
- The MCP 2026 roadmap acknowledges that notifications are "technically supported but not the aim of MCP protocol and LLMs are not designed to react on notifications from tools." The spec team is exploring explicit subscription streams and TTL-based caching as alternatives, with SEPs targeted for the June 2026 spec release.
- Other MCP clients (Cursor, Copilot) do not implement equivalent push mechanisms. Their MCP support is request-response: the AI calls a tool and gets a result.

**MCP is request-response, not event-driven.** The fundamental interaction model is: AI decides to call a tool, tool returns a result. This maps to bpmux's `read_messages` (poll) but not to "new message arrived, notify the AI" (push). The gap is structural, not just a missing feature. MCP was designed for tool use, not for channel participation.

**No session or presence concept.** MCP has no mechanism for an AI tool to know whether other participants are connected to bpmux, who they are, or when they last read messages. The server can track this internally (bpmux already does via WebSocket peer counting), but there is no standard way to expose it through MCP.

**Scalability gaps at the transport layer.** Running streamable HTTP MCP servers at scale has exposed issues: stateful sessions conflict with load balancers, horizontal scaling requires workarounds, and there is no standard discovery mechanism (no equivalent to A2A's Agent Cards for MCP servers).

### The honest assessment

MCP is necessary but not sufficient. It provides the universal integration layer -- any AI tool can connect to bpmux via MCP. But it does not provide the real-time, push-based participation model that Clark & Brennan's framework says is needed for conversational grounding. MCP gives bpmux reviewability (read the log on demand) but not cotemporality (receive messages as they arrive).

bpmux's current architecture already accounts for this. The bridge connects to bpmux via WebSocket (which provides cotemporality) and exposes MCP tools to the AI client (which provides the integration interface). The gap is the last mile: getting messages from the bridge into the AI's active context without the AI explicitly polling.

The degradation path documented in the channels risk assessment is the right mental model:

1. Channels working: real-time push (cotemporality for AI)
2. Channels broken, hooks working: per-turn sync (partial cotemporality)
3. Both broken: manual poll via `read_messages` (email-level constraint profile)

bpmux needs all three tiers because MCP alone only guarantees tier 3.

---

## 5. What Does Multi-Agent Framework Research Say About Heterogeneous Environments?

### The homogeneity assumption

CrewAI, AutoGen, and LangGraph -- the three dominant multi-agent frameworks as of 2026 -- all assume a homogeneous agent runtime:

- **CrewAI** uses a role-based model where all agents run within the same CrewAI process, share the same LLM backend configuration, and communicate through CrewAI's internal task output mechanism. Agents cannot use different LLM providers or run on different machines without custom integration.

- **AutoGen** uses a conversational collaboration model where agents participate in a GroupChat. Every agent turn involves a full LLM call with accumulated conversation history. All agents run within the same AutoGen runtime. Cross-language support exists but cross-platform agent participation does not.

- **LangGraph** uses a graph-based workflow model. Agents are nodes in a computation graph. The graph executes within a single LangGraph runtime. LangGraph reached v1.0 in late 2025 and became the default runtime for LangChain agents.

All three frameworks assume they control the agent lifecycle: they create agents, schedule their execution, manage their memory, and mediate their communication. None of them support a scenario where agents are running independently on different platforms and need to collaborate.

### What happens when agents run on different platforms

The research identifies several problems with heterogeneous agent environments:

**Communication model mismatch.** CrewAI agents communicate through task outputs (structured data). AutoGen agents communicate through conversation messages (natural language). LangGraph agents communicate through graph state transitions (key-value pairs). If a CrewAI agent and an AutoGen agent need to collaborate, they have no shared communication protocol. A2A is intended to bridge this gap, but adoption within these frameworks is nascent. OpenAgents claims LangGraph, CrewAI, and custom Python agents can participate in the same network via A2A, but this is edge-case integration, not the primary mode.

**State management divergence.** Each framework manages agent state differently. CrewAI has no built-in checkpointing. LangGraph has persistent state with thread-level isolation. AutoGen accumulates conversation history in memory. When agents from different frameworks participate in a shared conversation, there is no shared state model. Each agent maintains its own view of the conversation, potentially diverging.

**Scheduling and turn-taking conflicts.** CrewAI schedules agents sequentially (task pipeline) or hierarchically (manager delegates). AutoGen uses a GroupChat manager to select the next speaker. LangGraph follows graph edges. If agents from different frameworks participate in the same conversation, whose scheduling wins? The answer in practice: nobody's. The frameworks were not designed for shared scheduling.

**Token and latency economics.** AutoGen's GroupChat model is expensive: "every agent turn involves a full LLM call with accumulated conversation history -- a 4-agent debate with 5 rounds is 20 LLM calls minimum." CrewAI is cheaper per turn but coarser-grained. Different frameworks produce different cost profiles for the same collaboration pattern, making it hard to reason about the total cost of a heterogeneous session.

### What this means for bpmux

bpmux sidesteps the framework homogeneity problem by operating at a different layer. bpmux is not an agent framework. It does not create agents, schedule their execution, or manage their memory. It is a communication channel.

This is the same architectural insight that makes bpmux more general than ChatCollab. ChatCollab runs its own agent decision loop (poll, prompt, decide, act). bpmux delegates that to whatever tool the agent is running in. Claude Code has its own decision loop. Cursor has its own. Each framework manages its own agents. bpmux just carries the messages.

But the heterogeneity research identifies a real risk: **divergent state models**. If a Claude Code agent and a Cursor agent both read the bpmux log but maintain different internal representations of the conversation state, they may ground differently. Claude Code might have the full log in its context window. Cursor might have a summarized version. Their understanding of the conversation may diverge even though they read the same messages. bpmux cannot control this -- it is a property of the client tool's context management.

The mitigation is the one Clark & Brennan would predict: make grounding explicit. If agents are uncertain about shared understanding, they should use trial references, verbatim displays, and relevant next turns to verify alignment. The shared channel provides the medium for this verification. The agents (or their prompts) must provide the motivation.

---

## 6. Synthesis: The Cross-Tool Integration Problem for bpmux

### The problem, stated precisely

bpmux is a shared communication channel for heterogeneous AI participants. Different participants connect through different tools (Claude Code, Cursor, Copilot, etc.) with different capabilities (hooks, channels, polling). Clark & Brennan's framework predicts that heterogeneous constraint profiles produce asymmetric grounding costs. No existing research -- not ChatCollab, not the multi-agent framework literature, not the A2A or MCP specifications -- addresses this specific scenario.

### What the protocols provide

| Protocol | What it solves | What it does not solve |
|---|---|---|
| MCP | Universal tool integration (any AI tool can call `send_message` / `read_messages`) | Push notifications, presence, session awareness |
| A2A | Cross-vendor agent discovery, task delegation | Group conversation, shared timeline, grounding |
| CrewAI/AutoGen/LangGraph | Single-runtime agent orchestration | Cross-platform agent collaboration |
| WebSocket (bpmux-native) | Real-time push, presence tracking, broadcast | Last-mile delivery into AI tool context |

### bpmux's actual integration stack

The protocols layer as follows:

```
AI Tool (Claude Code, Cursor, etc.)
  |
  | MCP (tool interface: send_message, read_messages)
  |
Bridge Process (bpmux bridge)
  |
  | WebSocket (real-time, bidirectional, presence-aware)
  |
bpmux Server (shared session, JSONL persistence, broadcast)
```

MCP provides the universal top edge. WebSocket provides the real-time bottom edge. The bridge translates between them. The gap is the last mile from the bridge into the AI tool's active context -- and that gap varies by tool.

### What bpmux should do

**1. Accept the heterogeneity as permanent.** Different tools will always have different capabilities. Designing for the lowest common denominator (MCP poll) as the baseline, with progressive enhancement (hooks, channels) for tools that support more, is the right strategy. The degradation path already documented in the channels risk assessment is architecturally sound.

**2. Do not adopt A2A as a participation protocol.** A2A is a delegation protocol. It solves discovery and task assignment, not group conversation. If bpmux ever needs cross-network agent discovery, A2A's Agent Card concept is worth borrowing. But the ongoing collaboration protocol is bpmux's own WebSocket + MCP stack.

**3. MCP is the right integration layer, but push remains an unsolved problem.** The MCP 2026 roadmap acknowledges this. The subscription stream SEPs targeted for June 2026 may eventually provide a standard push mechanism. Until then, bpmux's bridge-based WebSocket-to-MCP translation is the pragmatic solution. Monitor the MCP spec evolution -- if subscription streams land, bpmux should adopt them to reduce dependence on Claude Code-specific channels.

**4. Invest in grounding affordances, not integration protocols.** The cross-tool problem is ultimately a grounding problem. Clark & Brennan predict that participants with different constraint profiles will need to compensate with explicit grounding techniques (trial references, verbatim displays, relevant next turns). bpmux can support this by providing the structural vocabulary the conduit model describes: `in_reply_to` fields, read cursors, and message type semantics. These affordances work regardless of which tool the participant uses, because they live in the message protocol, not the transport layer.

**5. Validate empirically.** No existing research covers bpmux's specific scenario: heterogeneous AI tools collaborating over a shared channel with asymmetric constraint profiles. ChatCollab validates the shared-channel model under homogeneous conditions. The multi-agent framework research validates that heterogeneity creates real problems. The intersection -- shared-channel collaboration under heterogeneous conditions -- is uncharted. bpmux will need to generate its own evidence.

---

## Sources

- [A2A Protocol Specification](https://a2a-protocol.org/latest/specification/)
- [Google Developers Blog: Announcing A2A](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)
- [A2A Protocol Getting an Upgrade (Google Cloud Blog)](https://cloud.google.com/blog/products/ai-machine-learning/agent2agent-protocol-is-getting-an-upgrade)
- [Linux Foundation Launches A2A Project](https://www.linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project-to-enable-secure-intelligent-communication-between-ai-agents)
- [The 2026 MCP Roadmap](http://blog.modelcontextprotocol.io/posts/2026-mcp-roadmap/)
- [Exploring the Future of MCP Transports](https://blog.modelcontextprotocol.io/posts/2025-12-19-mcp-transport-future/)
- [MCP vs A2A: Protocols for Multi-Agent Collaboration (OneReach)](https://onereach.ai/blog/guide-choosing-mcp-vs-a2a-protocols/)
- [A2A vs MCP: How These Protocols Actually Differ (DigitalOcean)](https://www.digitalocean.com/community/tutorials/a2a-vs-mcp-ai-agent-protocols)
- [Developer's Guide to AI Agent Protocols (Google)](https://developers.googleblog.com/developers-guide-to-ai-agent-protocols/)
- [MCP Transport Specification (2025-03-26)](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)
- [CrewAI vs LangGraph vs AutoGen (DataCamp)](https://www.datacamp.com/tutorial/crewai-vs-langgraph-vs-autogen)
- [OpenAgents: Open Source AI Agent Frameworks Compared](https://openagents.org/blog/posts/2026-02-23-open-source-ai-agent-frameworks-compared)
- [Multi-Agent Systems Survey (ACM DL)](https://dl.acm.org/doi/full/10.1145/3745238.3745531)
- [MCP Notifications Discussion (GitHub)](https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/1192)
