# A2A Protocol: Relevance to bpmux

*Researched April 2, 2026. Evaluating Google's Agent2Agent protocol.*

---

## What A2A Is

Agent2Agent (A2A) is an open standard for agent-to-agent communication, created by Google and donated to the Linux Foundation (Agentic AI Foundation). It complements MCP:

- **MCP** = agent-to-tool communication (agent calls tools/APIs)
- **A2A** = agent-to-agent communication (agents delegate tasks to each other)

## How It Works

- **Agent Cards** — JSON metadata describing identity, capabilities, skills, security. Agents discover each other through cards.
- **Task-based delegation** — clients send messages, agents return Tasks with unique IDs. Tasks move through states (working, input-required, completed, failed).
- **Multiple transports** — JSON-RPC 2.0, gRPC, HTTP/REST.
- **Opacity** — agents interact without exposing internal memory, tools, or logic.

## How bpmux Differs

| Aspect | A2A | bpmux |
|---|---|---|
| Model | Task delegation (client-server) | Shared conversation (P2P) |
| Visibility | Agents are opaque | Everything visible to everyone |
| Participants | Agents only | Humans AND agents |
| Communication | Structured tasks with state machines | Unstructured text messages |
| Discovery | Agent Cards | Session codes |
| Transport | HTTP, gRPC | WebRTC P2P |

**Key insight:** A2A is designed for agents delegating structured tasks to other agents across organizational boundaries. bpmux is designed for humans and agents sharing a conversation within a team. Different problems, complementary solutions.

## Where They Could Intersect

**v2+ consideration:** If bpmux evolves into a platform where Claude delegates tasks to local models (solo orchestration), the delegation protocol could speak A2A. The local model would be an A2A "remote agent" that Claude delegates to. The task lifecycle (assigned, working, completed) would flow through A2A, while the *reasoning* about that task flows through bpmux's shared chat.

**The complementary stack:**
- **MCP** — how each AI connects to bpmux (tool interface)
- **A2A** — how AIs delegate tasks to each other (structured delegation)
- **bpmux chat** — how all participants see what's happening (transparent shared environment)

MCP for tools. A2A for tasks. bpmux for understanding.

## Assessment for bpmux v1

**Not relevant for v1.** bpmux v1 is a message bus. There are no structured tasks, no delegation, no task state machines. A2A adds value when there's task decomposition and delegation — that's v2+ territory.

**Worth watching:** A2A is early (spec still evolving) and adoption is unclear. If it becomes the standard for agent-to-agent communication, bpmux should support it. If it fragments or stalls, bpmux's simple message protocol is sufficient.

**No spec changes needed.**

## Sources

- [A2A Protocol](https://a2a-protocol.org/latest/)
- [A2A Specification](https://a2a-protocol.org/latest/specification/)
