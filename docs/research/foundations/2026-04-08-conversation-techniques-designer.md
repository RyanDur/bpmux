# Conversation Techniques — Designer Research (Supplement)

*Designer research — 2026-04-08. Supplements PM's conversation-techniques.md with additional findings.*

## Findings Not Covered in PM's Research

### Token Duplication Crisis in Multi-Agent Systems

Peer-reviewed measurements of redundancy in multi-agent systems:
- MetaGPT: 72% token duplication rate
- CAMEL: 86% token duplication rate
- AgentVerse: 53% token duplication rate
- Multi-agent systems consume 1.5-7x more tokens than necessary

Cause: "specification ambiguity and unstructured coordination protocols cause agents to misinterpret roles and duplicate work." Systems fail at 41-86.7% rates in production due to coordination failures.

Source: Multi-Agent Collaboration Mechanisms Survey (2025), arXiv:2501.06322.

### Benne & Sheats' Functional Group Roles (1948)

26 roles in three categories. Key insight: when all group members play the same role (information giver), you get pile-on. Effective groups have differentiated roles:

- **Initiator-contributor:** proposes new ideas
- **Elaborator:** expands on suggestions with examples
- **Coordinator:** shows relationships among ideas, pulls together contributions
- **Gate-keeper:** manages communication flow, invites and moderates

For bpmux: PM initiates and routes, designer elaborates on UX, dev coordinates implementation. Each agent should know its *communication role*, not just its domain.

Source: Benne, K.D. & Sheats, P. (1948). "Functional Roles of Group Members." Journal of Social Issues, 4(2).

### Grice's Cooperative Principle (1975)

Four maxims for cooperative conversation:
- **Quantity:** Be as informative as required — no more. Pile-on violates this.
- **Relevance:** If another agent covered it, restating is irrelevant.
- **Manner:** Be brief; avoid unnecessary prolixity.
- **Quality:** Don't say what you believe to be false.

The quantity maxim is the formal check: "am I adding information, or restating?"

Source: Grice, H.P. (1975). "Logic and Conversation." Syntax and Semantics 3: Speech Acts.

### "Composing" Signals — The Quasi-Synchronous Gap

Text chat is quasi-synchronous (Herring 1999) — messages composed simultaneously arrive sequentially. This creates a structural invitation to pile-on that doesn't exist in face-to-face conversation. In voice, you hear someone start talking and hold back. In text, you can't.

Fix: broadcast "composing" status when an agent begins generating a response. Others see it and can wait or adjust.

Source: Herring, S.C. (1999). "Interactional Coherence in CMC." Journal of Computer-Mediated Communication, 4(4).

### "Who Speaks Next?" (Nonomura & Mori, 2025)

Directly applies Sacks/Schegloff/Jefferson's CA framework to multi-agent AI systems:
- Implements next speaker selection based on adjacency pairs
- Adds self-selection mechanism considering agents' internal states
- Result: "significantly reduced dialogue breakdowns and improved logical reasoning"

Source: Nonomura, R. & Mori, H. (2025). "Who Speaks Next?" Frontiers in Artificial Intelligence.

## Product Implication

bpmux could differentiate by being **turn-aware** — a medium that actively manages conversation flow:
- Messages carry addressee metadata (who should respond)
- "Composing" status prevents simultaneous generation
- Contribution tracking shows what aspects each agent has addressed
- Redundancy detection flags overlapping responses

No other chat tool occupies this position. Slack, Discord, and Teams are passive message displays. bpmux could be the first turn-aware group communication medium.
