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

## Failure Modes (What Went Wrong)

### File Conflicts
The #1 failure across all multi-agent systems. Multiple agents editing the same file leads to destructive overwrites. Even with git, agents restore old files and wipe out changes.

**bpmux implication:** v1 avoids this entirely — bpmux is a communication channel, not a code editor. For v2 (integrated coding tools), file ownership per agent or git worktree isolation is essential.

### Context Overflow
Single agents struggle with large codebases. Multiple agents each with their own context window help — but coordination between those windows is the hard part.

**bpmux implication:** This is bpmux's core value proposition. The shared message log IS the coordination layer between context windows.

### Coordination Bottleneck at the Human
Without agent-to-agent communication, the human becomes a relay — copying context between AI sessions. One developer described it as "managing a team of toddlers fighting over the same keyboard."

**bpmux implication:** This is exactly the problem bpmux solves. The current workaround (summarize context, paste into commit message) is a manual version of this relay. bpmux automates the relay.

### Verification Outpaced by Generation
The bottleneck in multi-agent systems is no longer code generation — it's verification. Agents produce code faster than it can be reviewed.

**bpmux implication:** The Ground Control / Major Tom working agreement addresses this. The human owns all decisions. TDD gates every change. The AI doesn't ship without human sign-off.

### Lost System Understanding
Delegating too much without oversight creates comprehension debt. If the human loses understanding of their own system, they lose the ability to fix it.

**bpmux implication:** The transparency principle ("everything is visible to everyone") directly counters this. The human sees all AI reasoning, all AI-to-AI communication. No hidden orchestration.

### Uncoordinated Work
Multiple agents without coordination primitives create duplicate work or conflicting approaches. One team observed all 16 agents hitting the same bug simultaneously.

**bpmux implication:** For v1, this is less relevant (2 peers, communication channel only). For v2+, task claiming and work partitioning become critical.

## Patterns That Work (What to Steal)

### Specialization Over Generalists
Three focused agents consistently outperform one generalist agent. Assign distinct roles (architect, implementer, reviewer) rather than having every agent do everything.

**bpmux application:** Already reflected in the working agreement (Opus architects, Sonnet implements, Haiku verifies) and the solo orchestration mode (Claude as architect, local model as implementer).

### Hierarchical Delegation
Feature leads spawning specialists prevents context fragmentation at the orchestrator level. The lead holds the big picture; specialists hold the details.

**bpmux application:** Maps to the v2 vision where Claude orchestrates local models. The bpmux chat is where the lead communicates plans and specialists report results.

### Quality Gates at Transitions
Plan approval before coding, hooks on task completion, human review before merge. The most reliable multi-agent systems gate transitions, not just outputs.

**bpmux application:** Already in the spec via TDD (tests gate implementation), pre-push hooks (quality gates shipping), and the working agreement (Ground Control approves before commit).

### Git Worktrees for Isolation
Giving each agent its own worktree eliminates merge conflicts entirely. True parallelism without coordination overhead.

**bpmux application:** Relevant for v2 when bpmux coordinates coding work. Not applicable for v1 (communication channel only).

### Compound Learning via Documentation
AGENTS.md / CLAUDE.md files that capture patterns, gotchas, and style rules so each session improves the next. Never AI-generated — human-curated.

**bpmux application:** Already in the spec (CLAUDE.md and AGENT_WORKFLOW.md as story 0 prerequisites). bpmux's persistent message log adds another layer — the AI reads the history of past decisions, not just static rules.

### Right-Size Teams (3-5 Agents)
Costs scale linearly, coordination overhead increases non-linearly. Beyond 5 agents, diminishing returns. 5-6 tasks per agent keeps everyone productive.

**bpmux application:** Relevant for v3 (teams). The 2-peer v1 limit is well within the effective range.

### Server-Side Task Coordination
Shared task lists with claiming, dependencies, and status tracking. File-lock-based claiming prevents race conditions.

**bpmux application:** Future feature for v2/v3. The shared notes concept (deferred from v1) is a lighter version of this.

## Key Insight

The industry is building multi-agent from the **solo developer** direction — one person, many AIs. Nobody is building from the **pair programming** direction — two people, each with AI, sharing context. These are different problems:

- Solo multi-agent: coordination is between AIs, human is orchestrator
- Pair programming with AI: coordination is between human-AI pairs, humans are collaborators

bpmux occupies unique territory. The closest existing tools (Claude Agent Teams, VS Code Multi-Agent) solve orchestration. bpmux solves collaboration.

## Sources

- [Claude Code Agent Teams](https://code.claude.com/docs/en/agent-teams)
- [VS Code Multi-Agent Development](https://code.visualstudio.com/blogs/2026/02/05/multi-agent-development)
- [Building a C compiler with parallel Claudes](https://www.anthropic.com/engineering/building-c-compiler)
- [The Code Agent Orchestra — Addy Osmani](https://addyosmani.com/blog/code-agent-orchestra/)
- [AI Agents Collaborate Like a Dev Team](https://medium.com/coding-nexus/how-i-got-my-ai-coding-agents-to-talk-plan-and-code-together-like-a-real-dev-team-8ccde98de886)
