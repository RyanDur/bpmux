# Multi-Agent Coding Landscape — Research Notes

*Compiled April 1, 2026, during bpmux v1 design.*

## What Exists

### Claude Code Agent Teams (Anthropic)
- One human orchestrates multiple Claude instances with a lead + teammates model
- Shared task list with file-lock-based claiming to prevent conflicts
- Direct teammate-to-teammate messaging via mailbox system
- Hooks for quality gates (TeammateIdle, TaskCreated, TaskCompleted)
- Split-pane display via tmux or iTerm2
- Experimental, known limitations: no session resumption, task status can lag, one team per session

### VS Code Multi-Agent Development (Microsoft)
- Unified session management for local, background, and cloud agents
- Parallel subagents that keep primary context clean
- Agent specialization by role (read-only research, editing implementation, security scanning)
- Open standards via MCP and Agent Skills

### Anthropic's Parallel Claudes (C Compiler Research)
- 16 Claude instances on shared codebase using git-based synchronization
- Task locking via text files in a `current_tasks/` directory
- No inter-agent communication — agents self-directed
- Early research prototype, not a product

### Windsurf
- Two engineers on the same codebase see AI suggestions together in real-time
- Closest to bpmux's pair mode but tied to Windsurf's editor
- AIs don't talk to each other

### Gas Town (Steve Yegge)
- Multi-agent workspace manager that orchestrates 20-30 Claude Code instances in parallel
- "The Mayor" is a Claude instance with full workspace context, you tell it what to do, it delegates
- Four failed iterations before the current design (TypeScript+Temporal → Go monolith → Python scripts → Go)
- Chose Go because AI models waste tokens on TypeScript's complex type system
- Key insight: "When work needs to be done, nature prefers colonies" — factories over super-agents
- Scaling remains unsolved: even experienced teams can't coordinate more than ~2-3 people at max agent velocity
- "Nobody knows how" to scale this to large companies — orgs are adopting "one engineer per repo" instead
- 100% vibe coded, 20 days old at launch, "rough but the clearest picture of orchestrated AI development"

### Beads (Steve Yegge)
- Persistent structured memory for coding agents — a git-backed JSONL issue tracker
- Solves inter-session amnesia: agents lose context between 10-minute sessions, Beads provides continuity
- Dependency-aware task graph: blocks, depends_on, relates_to, parent-child hierarchies
- `bd ready --json` lets agents query actionable work deterministically
- Multi-agent coordination via shared git repo with AI-assisted merge conflict resolution
- ~19k+ GitHub stars (as of April 2026), actively maintained
- Born from failure: Yegge's previous approach generated 605+ markdown plan files
- Gas Town was the fourth functioning orchestrator after multiple iterations (TS/Temporal failed, TS produced Beads, Python, then Go/Gas Town)
- Core insight (from Claude): "Beads isn't issue tracking for agents — it's external memory for agents"

## Failure Modes (What Went Wrong)

### File Conflicts
The most commonly reported failure across multi-agent systems we surveyed. Multiple agents editing the same file leads to destructive overwrites. Even with git, agents restore old files and wipe out changes.

**bpmux implication:** v1 avoids this entirely — bpmux is a communication channel, not a code editor. For v2 (integrated coding tools), file ownership per agent or git worktree isolation is essential.

### Context Overflow
Single agents struggle with large codebases. Multiple agents each with their own context window help — but coordination between those windows is the hard part.

**bpmux implication:** This is the problem bpmux aims to solve. Our hypothesis: a shared message log can serve as the coordination layer between context windows. This is untested — dogfooding will validate or invalidate it.

### Coordination Bottleneck at the Human
Without agent-to-agent communication, the human becomes a relay — copying context between AI sessions. One developer described it as "managing a team of toddlers fighting over the same keyboard."

**bpmux implication:** This is exactly the problem bpmux solves. The current workaround (summarize context, paste into commit message) is a manual version of this relay. bpmux automates the relay.

### Verification Outpaced by Generation
The bottleneck in multi-agent systems is no longer code generation — it's verification. Agents produce code faster than it can be reviewed.

**bpmux implication:** bpmux provides visibility into AI reasoning (the human can see what the AI is doing), but it doesn't enforce verification gates. That's a team process decision, not a tool feature. A team using bpmux without TDD or review discipline would still face this risk.

### Lost System Understanding
Delegating too much without oversight creates comprehension debt. If the human loses understanding of their own system, they lose the ability to fix it.

**bpmux implication:** The transparency principle ("everything is visible to everyone") directly counters this. The human sees all AI reasoning, all AI-to-AI communication. No hidden orchestration.

### Uncoordinated Work
Multiple agents without coordination primitives create duplicate work or conflicting approaches. One team observed all 16 agents hitting the same bug simultaneously.

**bpmux implication:** For v1, this is less relevant (2 peers, communication channel only). For v2+, task claiming and work partitioning become critical.

### Plan Proliferation (Beads)
Yegge's pre-Beads approach generated 605+ markdown plan files. Agents create nested plans that recurse endlessly, forget outer context, and restart planning from scratch at deeper levels. Markdown doesn't scale as a planning medium for agents.

**bpmux implication:** bpmux's message log is a stream, not a planning document. This avoids plan proliferation by design. But if we ever add structured task management (v2+), Beads' graph-based approach is the proven alternative to markdown plans.

### Speed Creates Coordination Problems (Gas Town)
At maximum agent velocity, 2-hour-old information is "ancient." Coordination becomes essential, not optional. Even experienced teams can't effectively coordinate more than 2-3 people at agent-assisted speeds. Organizations are giving up on team scaling and going "one engineer per repo."

**bpmux implication:** This is a direct validation of bpmux's reason to exist. If coordination breaks down at speed, and the industry is defaulting to solo work as a workaround, then a tool that makes coordination cheap is high value. bpmux's real-time P2P chat is faster coordination than git-based async.

### Orchestrator Iteration (Gas Town)
Yegge went through four failed orchestrator designs before Gas Town worked: TypeScript+Temporal (too heavyweight), Go monolith (wrong problem), Python scripts (too ad-hoc), then finally a Go implementation. The lesson: orchestration is hard to get right, and simpler is better.

**bpmux implication:** v1 is deliberately simple — a message bus, not an orchestrator. This is the right starting point. Let the orchestration patterns emerge from usage rather than designing them upfront.

## Patterns That Work (What to Steal)

### Specialization Over Generalists
Practitioners report (Osmani, Yegge, Claude Code docs) that focused agents outperform generalist agents. Assign distinct roles (architect, implementer, reviewer) rather than having every agent do everything. This is practitioner consensus, not a controlled study.

**bpmux application:** bpmux's multi-MCP-client support enables connecting multiple specialized AIs to the same session. How those AIs are specialized (by role, by task, by domain) is a team process decision, not a bpmux feature.

### Hierarchical Delegation
Feature leads spawning specialists prevents context fragmentation at the orchestrator level. The lead holds the big picture; specialists hold the details.

**bpmux application:** Maps to the v2 vision where Claude orchestrates local models. The bpmux chat is where the lead communicates plans and specialists report results.

### Quality Gates at Transitions
Plan approval before coding, hooks on task completion, human review before merge. The most reliable multi-agent systems gate transitions, not just outputs.

**bpmux application:** Quality gates (TDD, pre-push hooks, review discipline) are team process decisions. bpmux's contribution is making AI reasoning visible so that whatever review process the team uses has the full context available.

### Git Worktrees for Isolation
Giving each agent its own worktree eliminates merge conflicts entirely. True parallelism without coordination overhead.

**bpmux application:** Relevant for v2 when bpmux coordinates coding work. Not applicable for v1 (communication channel only).

### Compound Learning via Documentation
AGENTS.md / CLAUDE.md files that capture patterns, gotchas, and style rules so each session improves the next. Never AI-generated — human-curated.

**bpmux application:** Already in the spec (CLAUDE.md and AGENT_WORKFLOW.md as story 0 prerequisites). bpmux's persistent message log adds another layer — the AI reads the history of past decisions, not just static rules.

### External Memory Over Markdown Plans (Beads)
Structured, queryable task storage beats markdown files. Agents need to query "what's ready to work on?" deterministically, not parse a plan document. Git-backed JSONL with dependency tracking gives agents a database they can reason about.

**bpmux application:** The persistent message log is bpmux's version of external memory — but for conversations, not tasks. If bpmux adds task management (v2+), Beads' architecture (git-backed, dependency-aware, query-friendly) is the model to follow. Could even integrate with Beads directly rather than building a competing task system.

### Transparency at Velocity (Gas Town)
When agents move fast, everything must be visible. Yegge insists on workflows that are "all transparent, announced, all the time." Hidden agent work at high velocity leads to conflicts and wasted effort.

**bpmux application:** Already a core principle — "everything is visible to everyone." Validated by Gas Town's experience at scale.

### Simplicity in Agent Code (Gas Town)
Yegge chose Go over TypeScript because AI models waste tokens on complex type systems. Simple code is easier for agents to understand and generate correctly.

**bpmux application:** Rust is more complex than Go, but bpmux's agent-facing surface is simple: MCP tools with clear contracts. The complexity is in the transport layer (WebRTC), not in the AI-facing interface.

### Right-Size Teams
Claude Code docs recommend 3-5 teammates; Wills ran ~20 agents but reported cognitive fatigue after 3 hours (n=1). Google DeepMind's 2025 research ("Towards a Science of Scaling Agent Systems," 180 agent configurations) found that more agents often degrade performance on sequential tasks and that unstructured agent teams can amplify errors. No study identifies a specific optimal agent count — "3-5" is practitioner heuristic, not a research finding.

**bpmux application:** Relevant for v3 (teams). The 2-peer v1 limit sidesteps the scaling question entirely.

**Source:** [Google DeepMind — Towards a Science of Scaling Agent Systems](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/)

### Server-Side Task Coordination
Shared task lists with claiming, dependencies, and status tracking. File-lock-based claiming prevents race conditions.

**bpmux application:** Future feature for v2/v3. The shared notes concept (deferred from v1) is a lighter version of this.

## Key Insight

The industry is building multi-agent from the **solo developer** direction — one person, many AIs. Nobody is building from the **pair programming** direction — two people, each with AI, sharing context. These are different problems:

- Solo multi-agent: coordination is between AIs, human is orchestrator
- Pair programming with AI: coordination is between human-AI pairs, humans are collaborators

Based on our survey, no existing tool provides P2P communication between two developer-AI pairs. The tools we found (Claude Agent Teams, VS Code Multi-Agent, Gas Town) solve orchestration — one human directing multiple AIs. bpmux aims to solve collaboration — multiple humans with AIs coordinating together. Beads solves memory. These appear complementary, not competing — bpmux could integrate with Beads for task tracking while providing the real-time communication channel.

Yegge's observation that "nobody knows how to scale coordination to large companies" and the industry default of "one engineer per repo" suggests coordination is an unsolved problem. Our hypothesis: if coordination is the bottleneck, a tool that makes it cheaper has value. This is a bet, not a validated finding.

## Sources

- [Claude Code Agent Teams](https://code.claude.com/docs/en/agent-teams)
- [VS Code Multi-Agent Development](https://code.visualstudio.com/blogs/2026/02/05/multi-agent-development)
- [Building a C compiler with parallel Claudes](https://www.anthropic.com/engineering/building-c-compiler)
- [The Code Agent Orchestra — Addy Osmani](https://addyosmani.com/blog/code-agent-orchestra/)
- [AI Agents Collaborate Like a Dev Team](https://medium.com/coding-nexus/how-i-got-my-ai-coding-agents-to-talk-plan-and-code-together-like-a-real-dev-team-8ccde98de886)
- [Welcome to Gas Town — Steve Yegge](https://steve-yegge.medium.com/welcome-to-gas-town-4f25ee16dd04)
- [The Future of Coding Agents — Steve Yegge](https://steve-yegge.medium.com/the-future-of-coding-agents-e9451a84207c)
- [Introducing Beads — Steve Yegge](https://steve-yegge.medium.com/introducing-beads-a-coding-agent-memory-system-637d7d92514a)
- [Gas Town GitHub](https://github.com/steveyegge/gastown)
- [Beads GitHub](https://github.com/steveyegge/beads)
