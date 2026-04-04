# Contradiction Register

Disagreements between team members (researcher, designer, PM) that were not resolved by consensus. These are signal, not noise. Each contradiction represents a tension in the product that may need revisiting as we learn more.

---

## 1. Compressed understanding: new problem or existing?

**Researcher says:** It's a new (8th) problem. The gap between reviewability and understanding is real and distinct from cognitive debt. Research supports it (workspace awareness, shared mental models).

**Designer says:** It's not new — it's cognitive debt (#2) experienced in real-time. Don't add a new category. Test with `/status` command before committing to the framing.

**Resolution:** Half-adopted. Named the gap but framed as real-time manifestation of #2, not a new problem. Could be revisited if `/status` testing reveals it's genuinely distinct.

**What would change our mind:** If users report "I can read the log but I still don't know what's happening" — that's the compressed understanding gap being distinct from cognitive debt.

---

## 2. Read cursors: useful or noise?

**Researcher says:** Add to protocol. Clark & Brennan's acceptance phase requires evidence of consumption. Read cursors provide that evidence.

**Designer says:** "They answer a question users aren't asking." What the human needs is liveness (is the AI engaged?), not read state (did the TCP ACK arrive?). Slack's read receipts are widely disliked. Keep on transport layer, surface only as status bar liveness indicator.

**PM says:** Track internally, don't expose per-message. The designer is right about the UX, the researcher is right about the theory.

**Resolution:** Tracking internally, not rendering per-message. May revisit if grounding problems emerge in multi-user sessions.

**What would change our mind:** If users say "I didn't know the agent saw my message" — read cursors become a real need, not a theoretical one.

---

## 3. When to build for the surveillance risk

**Researcher says:** Document as prerequisite now. "bpmux requires psychological safety to function." The risk is real and well-documented (Edmondson).

**Designer says:** Not a concern with one human. Build `/draft` toggle only when second human joins. Don't build access controls, ephemeral messages, or anything for an adversarial trust model.

**Resolution:** Documented as prerequisite (thesis v2). No features built yet. `/draft` toggle planned for when second human joins.

**What would change our mind:** If during user study Phase 2 (exit interviews), users report self-censoring or say "I didn't want to ask a dumb question in the log."

---

## 4. "Shared reasoning environment" as product framing

**Researcher says:** Half-adopt. "Collective memory for human-AI teams" is better than "shared reasoning environment" (too abstract) but better than "P2P group chat" (undersells the value).

**Designer says:** Marketing language until a feature demands it. The thing you built is a group chat. The thing you use it for is shared reasoning. Don't reorganize code around the label.

**Resolution:** Use "collective memory for human-AI teams" externally. Don't rename internally. The reframing becomes real only when we add the first feature a group chat wouldn't have.

**What would change our mind:** If a concrete feature emerges that only makes sense under the "shared reasoning environment" framing, the label becomes load-bearing.

---

## 5. Observer vs inquiry for user study

**Researcher says:** Silent observer first. "You get one chance to see unobserved behavior." JSONL logs are the primary instrument. Don't contaminate natural behavior.

**Designer says:** Active inquiry with 5-question exit interviews. Silent observation doesn't work for TUIs — you can't see what someone tried and failed, only what they typed.

**Resolution:** Both, sequenced. Phase 1 is silent (logs only). Phase 2 is 5-question exit interview after 2-3 sessions. The uncontaminated sessions are captured in the log before we ever ask.

**What would change our mind:** If Phase 1 logs reveal nothing actionable, skip to Phase 2 sooner. If Phase 2 interviews change user behavior noticeably, the researcher's concern was justified.

---

## 6. A2A protocol as integration path

**Market researcher says:** A2A is the next integration target. 50+ launch partners, Linux Foundation backing. The protocol ecosystem is stabilizing around MCP + A2A.

**Cross-tool researcher says:** A2A is irrelevant for bpmux. "A2A is a delegation protocol (send task, get result). bpmux needs a participation protocol (peers observe and contribute to a shared timeline). A2A has no shared channel concept."

**Resolution:** A2A is useful for agent discovery (Agent Cards) but not for ongoing collaboration. bpmux's own WebSocket protocol is the participation layer. MCP is the tool integration layer. A2A solves a different problem.

**What would change our mind:** If A2A adds a group session or shared channel concept in a future spec version.

---

## 7. Truncation threshold

**Session analysis:** Problem starts at 5 wrapped lines (390 chars at 78 columns).

**Design recommendation:** Truncate at 3 lines of visible content + `[+N lines]`.

**Tension:** 3 is aggressive — shows less than half of a "problem" message. But the designer argues the point is information density in an 80x24 terminal, not preserving maximum visible text.

**Resolution:** Design at 3, implement as configurable if possible, validate with users.

**What would change our mind:** If users complain about truncation being too aggressive, raise to 5. If they don't notice, 3 was right.

---

## 8. Ctrl-U/D keybinding conflict

**Current implementation:** Ctrl-U/D for vim-style half-page scroll.

**Terminal convention:** Ctrl-U is readline "kill line." Ctrl-D is "delete char" / "EOF."

**Tension:** Vim users expect scroll. Readline users (every developer in a terminal) expect line editing. Both are "terminal conventions" but they conflict.

**Resolution:** Unresolved. Added as Question #6 in tech lead briefing. Need to decide: context-dependent (input mode vs chat mode), or pick one convention.

**What would change our mind:** User study data showing which binding developers actually try first.

---

*Last updated: April 4, 2026. Add new contradictions as they arise. Do not resolve by deleting — update the resolution and "what would change our mind" sections.*
