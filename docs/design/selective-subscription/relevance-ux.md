# Selective-subscription — relevance-UX (designer half, #2 owner-bundle)

Pair-shape with PM. PM-half (owner-anchor + user-job + falsifier +
delivery-gap) seeded in the pm/designer/ryan room. This is the
designer relevance-UX half + the forks for PM convergence. Output =
the UX + a buildable story body; PM files after we converge.

**Owner-anchor (Ryan, verbatim):** "aware but not a firehose —
subscribe to what you NEED, not over-saturated."

## The grounding that reframes the work

There are TWO delivery paths to an agent's LLM stream:

1. **Structured events — ALREADY subscription-gated.** The 5 event
   classes (`board_invalidated`, `ci_state_changed`,
   `sync_status_changed`, `story_state_transition`, `addressed_to_me`)
   are dispatched only to subscribers via `wait_for_event`
   (`event_subscription.rs`, `connection.rs:113-171`). Not a firehose.
2. **Chat broadcast — NOT subscription-gated.** Every connected agent's
   bridge pushes ALL chat to its LLM (`connection.rs:172-236`), minus
   only `message_type_reaches_llm_stream` (drops `activity`/`system`) +
   addressee-relevance. **This is the firehose.**

So "aware but not a firehose" is about the **chat-broadcast path**. The
subscribe MECHANISM already exists (subscribe/unsubscribe/
wait_for_event + #1631 persistence/replay/cursor) — the gap is that
chat delivery doesn't consult it. Dev-grounded: the fix is a per-agent
**chat-delivery-filter**, sibling to #1525 (nudge-exclude-at-ingest) +
#1643 (addressed_to_me already-filtered).

**Realized value of the #1631 keystone — not a new firehose mechanism.**

## (1) How an agent expresses its needed classes

A declarative **relevance-set**: the classes of traffic the agent wants
PUSHED to its LLM stream. Expressed via the existing `subscribe`
surface, extended so chat-classes are subscribable alongside the 5
structured event-classes — one relevance vocabulary across both paths.

The vocabulary an agent picks from:
- **Chat axis:** addressed-to-me · owner(Ryan)-broadcasts ·
  broadcast-chat · by-channel/topic · by-sender-role · by-message-class
  (events / bpmux / insight / nudge)
- **Structured axis:** the 5 event-classes (already subscribable)
- **Filters** (already supported — exact key-value match on payload):
  e.g. `story_state_transition {target_state: "ready"}`,
  `addressed_to_me {addressee: "designer"}`

**Anti-starvation default-set** (the heart of "not over-saturated"):
the relevance-set is NOT empty-by-default and NOT firehose-by-default.
A curated default pushes the must-haves — addressed-to-me +
owner-direct + your-domain — and suppresses the rest. The default IS
the not-over-saturated baseline; agents tune from there. Must-haves
(addressed-to-me, owner-direct) are **un-muteable** so an agent can't
accidentally starve itself (falsifier clause 3).

## (2) The manage / inspect surface

A legible **relevance-profile** — "what am I receiving / what's
suppressed":
> designer receives: addressed_to_me · ryan-broadcasts ·
> story_state_transition(ready) · chat-broadcast
> suppresses: activity · ci_state_changed · nudges-for-others
- Agent reads + edits its own profile via subscribe/unsubscribe (exist).
- **Team-observability tie-in (#1605):** Ryan can inspect each agent's
  relevance-profile — *that* is the "aware" surface for the owner: he
  sees what each agent is tuned to, so the team's attention-routing is
  legible, not hidden.

## (3) What "not-over-saturated" reads like in the stream

The duality that makes it "aware but not a firehose":
- **Not-saturated:** the PUSH-stream (what reaches the LLM context)
  carries only relevance-set traffic. Suppressed traffic burns no
  context.
- **Still-aware:** suppressed traffic stays in the LOG, is catchable
  on-demand via `wait_for_event`, and is replayable via the #1631
  cursor. Nothing is lost — it's just not auto-pushed.

Symmetry worth noting: the renderer already has a human VIEW-filter
(chat/events/bpmux via `useMessageFilter.ts`). The relevance-set is the
agent analog — the human filters their *view*; the agent filters its
*push-stream*. Same taxonomy, two surfaces.

## Forks for PM convergence

1. **Default-set composition** — exact must-have list (addressed-to-me +
   owner-direct + ? domain-relevance). What's un-muteable.
2. **Opt-in vs opt-out vs curated-default** — designer-lean:
   curated-default (safe must-haves on, firehose off, tune from there).
   Literal "subscribe to what you NEED" but never-starve.
3. **First buildable slice boundary** — designer-lean: the per-agent
   chat-delivery-filter consulting the relevance-set (the #1525-sibling
   vertical) is slice 1. Manage/inspect-profile + Ryan-observability
   (#1605) are follow-on slices, not slice 1.
4. **Vocabulary unification** — do chat-classes + the 5 event-classes
   live under ONE subscribe surface (designer-lean: yes, one relevance
   vocabulary), or stay separate?
5. **Relation to #1643** — addressed_to_me filtering already ships; the
   relevance-set generalizes that pattern. Confirm #1643 is the
   precedent slice, not a duplicate.

## Observation-falsifier (inherits PM-half)

Reject if: a subscribed agent still receives chat OUTSIDE its
relevance-set (firehose not actually suppressed for it); OR there's no
surface to express which classes it needs; OR the filter is
over-aggressive and starves it of a must-have (addressed-to-me /
owner-direct must be un-muteable).
