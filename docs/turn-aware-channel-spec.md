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

**`addressee` is distinct from the existing `to` field.** The `to` field is a **hard delivery filter** used by `/to <role>` DMs — if a role is not in `to`, the bridge does not deliver the message to them at all. That is privacy/routing.

`addressee` is a **soft attention signal** layered on top: the bridge still delivers the message to every recipient listed in `to` (or everyone, when `to` is None), but indicates which role is expected to respond. Non-addressed agents see the message in their log and can skip processing it.

| Field | Type | Role |
|-------|------|------|
| `to` | `Option<Vec<String>>` | Privacy filter — which roles can SEE the message. Preserves existing DM semantics. |
| `addressee` | `Option<String>` | Attention signal — which role is expected to RESPOND. Every delivered agent still receives the message; only processing is gated. |

The two fields compose. A broadcast directed at PM looks like `to: None, addressee: "pm"` — everyone sees it, PM processes it as their turn. A private DM to PM looks like `to: ["pm"], addressee: None` — only PM sees it, and there is no attention metadata because privacy already implies who should respond.

**UX impact:** When Ryan types in the PM channel, only PM processes the message. Designer and dev see it in their log but don't process/respond unless it's relevant to them.

**Token savings:** 2/3 reduction on directed messages. Only the addressed agent burns inference tokens.

**Implementation note:** the `Message` struct should carry explicit doc comments on both fields to prevent future readers from conflating the two semantics. Example:

```rust
pub struct Message {
    /// Privacy filter: recipients who can SEE this message.
    /// `None` = broadcast to all peers. `Some(vec)` = restricted
    /// to the listed roles (used by `/to <role>` DMs).
    pub to: Option<Vec<String>>,

    /// Attention signal: role this message is DIRECTED at.
    /// Every delivered agent still receives the message; this field
    /// only indicates which role is expected to respond. Non-addressed
    /// agents see the message in their log but skip processing.
    pub addressee: Option<String>,
    // ...
}
```

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
