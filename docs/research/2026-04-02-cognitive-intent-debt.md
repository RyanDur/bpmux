# Cognitive Debt and Intent Debt: The Theoretical Foundation for bpmux

*The missing framework. Storey's Triple Debt Model names the problem bpmux was designed to solve.*

---

## The Triple Debt Model (Storey, 2026)

Margaret-Anne Storey's paper "From Technical Debt to Cognitive and Intent Debt: Rethinking Software Health in the Age of AI" (arXiv:2603.22106, March 2026) proposes that AI-accelerated development creates two under-appreciated forms of debt alongside traditional technical debt:

**Technical debt** lives in the code. The traditional concern: shortcuts, complexity, missing tests.

**Cognitive debt** lives in people. The erosion of shared understanding across a team. When AI generates code faster than teams can comprehend it, the understanding fragments. Team members can no longer articulate *why* design decisions were made or *how* system components interact. As Storey writes, building on Peter Naur's insight: "a program is a theory" residing in developers' collective understanding. When that theory fragments, the team loses the ability to safely change the system.

**Intent debt** lives in (absent) documentation. The absence of externalized rationale that both developers and AI agents need to work safely with code. The code may be clean, the tests may pass, and yet the *why* is nowhere to be found. Intent debt is what makes cognitive debt permanent — if the rationale isn't captured somewhere, it can't be recovered when team members change or AI sessions restart.

## The Key Insight

AI may *reduce* technical debt (cleaner code, better tests, automated refactoring) while simultaneously *accelerating* cognitive and intent debt. The code is fine. Nobody understands why it's that way.

Storey identifies this as the fundamental shift: "the debt compounded from going fast lives in the brains of the developers." Speed without understanding is the new risk.

## Warning Signs (from Storey)

- Team members hesitating to make changes due to fear of unintended consequences
- Reliance on "tribal knowledge" held by just one or two people
- The system becoming perceived as a "black box"

## Storey's Recommended Mitigations

1. Require human understanding — at least one team member must fully comprehend each AI-generated change
2. Document intentionality — record not just *what* changed but *why*
3. Rebuild shared understanding — code reviews, retrospectives, knowledge-sharing sessions
4. Embrace deliberate practices — pair programming, refactoring, TDD (Kent Beck: "make the hard change easy")

## How bpmux Maps to the Triple Debt Model

This is not a post-hoc rationalization. bpmux was designed to solve the problems Storey names — we just didn't have her vocabulary for it until now.

### Cognitive Debt Reduction (bpmux as a tool)

Cognitive debt is the erosion of shared understanding. bpmux's design directly addresses this:

| Storey's Warning Sign | bpmux Feature | How It Helps |
|---|---|---|
| Team members afraid to make changes | Transparent chat — AI reasoning is visible | The *why* behind changes is observable, reducing fear of the unknown |
| Tribal knowledge in one person | Persistent shared log across all participants | Understanding is distributed through the chat, not locked in one person's head (or one AI session) |
| System as black box | AI-to-AI communication visible to humans | When AIs discuss the codebase, their understanding becomes part of the shared record |

Storey says cognitive debt must be repaid by maintaining "distributed knowledge across people, documentation, tests, conversations, tooling, and increasingly, AI agents." bpmux is the *conversations* and *AI agents* layer of that list.

### Intent Debt Reduction (bpmux as a tool)

Intent debt is the absence of externalized rationale. bpmux's persistent message log captures rationale as a side effect:

- When the AI explains why it chose an approach → intent is captured
- When the human questions a decision → the reasoning and counter-reasoning are captured
- When the pair discusses tradeoffs → the tradeoff analysis is captured

No one has to write documentation. The conversation IS the intent record. This is why we identified the log as a "decision journal" — research on decision journaling shows that recording reasoning improves future decision quality. Storey's framework explains *why*: it prevents intent debt from accumulating.

### The Working Agreement (process, not tool)

Storey's first recommendation: "Require human understanding — at least one team member must fully comprehend each AI-generated change." bpmux makes this possible by keeping reasoning visible. Whether the team actually requires understanding before shipping is a process decision, not a tool feature.

Storey's fourth recommendation: "Embrace deliberate practices — pair programming, refactoring, TDD." These are team process choices. bpmux's contribution: the persistent log captures the reasoning that informs those practices, and the transparent chat makes pair programming across AI sessions possible.

### Stigmergy and Debt

Under the stigmergy framing, cognitive debt is what happens when the shared environment doesn't preserve enough traces. If agents modify the code (sematectonic traces) without leaving signals about *why* (sign-based traces), the environment becomes opaque. Other agents can see *what* changed but not *why* — exactly Storey's description of intent debt.

bpmux adds sign-based traces (messages about reasoning) alongside the sematectonic traces (code changes). Both are needed for a healthy stigmergic environment. Code without reasoning is intent debt. Reasoning without code is empty talk. The combination is what prevents debt accumulation.

## What This Changes for bpmux

### The value proposition is clearer

"P2P group chat for AI pair programming" is a *mechanism* description. "Cognitive and intent debt reduction for AI-assisted development" is a *value* description. The mechanism is the same — persistent shared chat with transparency. But the value framing connects to a named, research-backed problem that every team using AI will face.

### The dogfooding questions sharpen

The v1 spec already names four hypotheses to test. Storey's framework adds specific metrics to watch:

- **Cognitive debt indicator:** Can you explain why the code is the way it is after a session? Can your AI? Can your partner's AI? If the answer is "read the bpmux log," that's cognitive debt being prevented. If the answer is "I don't know, it just works," that's cognitive debt accumulating.
- **Intent debt indicator:** When a new AI session starts, does reading the bpmux log give it enough rationale to make safe changes? Or does it just see *what* happened without understanding *why*?

### Solo mode value is validated

Storey says intent debt is what makes cognitive debt permanent — if rationale isn't captured, it can't be recovered. bpmux's solo mode (persistent log between you and your AI) is intent debt prevention for a single developer. You don't need a partner to benefit. Every AI session that captures its reasoning in the bpmux log is preventing intent debt that would otherwise accumulate silently.

## Sources

- [Storey (2026) — From Technical Debt to Cognitive and Intent Debt (arXiv:2603.22106)](https://arxiv.org/abs/2603.22106)
- [Storey blog — How Generative and Agentic AI Shift Concern from Technical Debt to Cognitive Debt](https://margaretstorey.com/blog/2026/02/09/cognitive-debt/)
- [Storey blog — What I'm Hearing About Cognitive Debt (So Far)](https://margaretstorey.com/blog/2026/02/18/cognitive-debt-revisited/)
- [Simon Willison on cognitive debt](https://simonwillison.net/2026/Feb/15/cognitive-debt/)
