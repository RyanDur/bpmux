# Turn-Aware Channel — UX Spec

*Designer spec for PM to shape into stories. Addresses pile-on, token efficiency, and conversation quality.*

## Problem

Three AI agents in a shared channel all process every message and respond simultaneously. This causes:
- **Pile-on:** 3 overlapping responses to 1 message
- **Token waste:** MetaGPT measured 72% duplication, CAMEL 86% (arXiv:2501.06322)
- **Poor conversation quality:** parallel monologues instead of building dialogue

## Design Direction

Make bpmux **turn-aware** — the first group chat medium that actively manages conversation flow.

## Three Layers

### Layer 1: Message Metadata (Infrastructure)

Messages carry:
- `addressee`: who should respond (role name, "all", or omitted for broadcast)
- `channel`: which view the sender is in (shipped in #92)

**UX impact:** When Ryan types in the PM channel, only PM processes the message. Designer and dev see it in their log but don't process/respond unless it's relevant to them.

**Token savings:** 2/3 reduction on directed messages. Only the addressed agent burns inference tokens.

### Layer 2: Composing Signals (Awareness)

When an agent starts generating a response, bpmux broadcasts a "composing" status. Other agents see it and can:
- Wait to see what the first responder says
- Decide if they have something to add
- Skip responding entirely if the domain owner is handling it

**UX impact:** Status bar shows "● designer composing..." in yellow/italic. Same area as current status dots.

**Token savings:** Prevents simultaneous generation. Agent 2 sees "designer is composing" and holds — doesn't start its own response.

### Layer 3: Contribution Tracking (Intelligence)

After the domain owner responds, bpmux can surface: "designer covered [UX, accessibility]. dev: anything to add on [implementation]?"

**UX impact:** This is a facilitation prompt — bpmux acts as gate-keeper (Benne & Sheats 1948), routing the conversation to uncovered angles.

**Token savings:** Agents only respond when they have genuinely new information to add. No echoing.

## Priority

Layer 1 is highest priority — addressee metadata has the biggest token impact with the simplest implementation. Layer 2 requires bridge protocol changes. Layer 3 requires intelligence/analysis and is exploratory.

## What This Is NOT

- Not threading (we rejected threading — it fragments awareness)
- Not a speaking queue (agents can still choose to speak)
- Not permission-based (agents aren't blocked from responding, just informed)
- Not AI orchestration (bpmux carries messages, doesn't direct agents)

## Constraints

- Must work for any AI tool, not just Claude (Cursor, Copilot, etc.)
- Must not add latency to message delivery
- Must degrade gracefully (if metadata is missing, behave like current broadcast)
- Must not require agents to implement a complex protocol
