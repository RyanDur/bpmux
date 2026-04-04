# System Prompt Design for AI Coding Assistants in Group Chat

Deep research into how production AI coding tools construct their system prompts, what academic work says about multi-party AI participation, and what this means for bpmux.

## 1. Claude Code's System Prompt: Reverse-Engineered

Source: [Piebald-AI/claude-code-system-prompts](https://github.com/Piebald-AI/claude-code-system-prompts) (updated within minutes of each Claude Code release, currently tracking v2.1.91).

### The Scale

Claude Code does not have one system prompt. It has **254 separate prompt files** organized into five categories:

| Category | File Count | Purpose |
|---|---|---|
| Agent Prompts | 33 | Sub-agent behavior (Plan, Explore, Task, CLAUDE.md creation, security review) |
| Data Reference | 17 | API docs, SDK patterns, HTTP codes, tool use guides |
| Skill Files | 13 | Agent design patterns, debugging workflows, config updates |
| System Prompts | 118 | Core behavioral instructions, mode specs, tool policies, tone guidance |
| System Reminders | 53 | Context-aware notifications injected mid-session |
| Tool Descriptions | 68 | Specifications for each built-in tool |

These are **conditional**. The prompt assembler selects which pieces to include based on environment, active mode, tool availability, and session state. A given session might see 20-30 of these; it never sees all 254.

### The Layers

**Layer 1: Identity and Output Style.** The outermost layer establishes who the agent is and how it communicates:

> "You are Claude Code, Anthropic's official CLI for Claude. You are an interactive CLI tool that helps users with software engineering tasks."

The output style is aggressively brief:
- "You MUST answer concisely with fewer than 4 lines" unless detail is requested
- "minimize output tokens as much as possible"
- One-word answers preferred where applicable
- No emojis unless explicitly requested
- No framing devices ("The answer is...", "Here is what I will do next...")

This is a deliberate anti-pattern correction. The default LLM behavior is verbose and over-helpful. Claude Code's prompt fights that directly.

**Layer 2: Safety and Security.** Embedded in the core prompt, not bolted on:
- "Assist with defensive security tasks only. Refuse to create, modify, or improve code that may be used maliciously."
- Never expose secrets or keys
- Never commit unless user requests it
- Run lint/typecheck after code changes

**Layer 3: Tool Use Policies.** Each of the 68 tool descriptions includes not just what the tool does but *when to use it and when not to*. The Bash tool alone has 40+ related prompt files covering sandbox behavior, git safety, and parameter handling.

**Layer 4: Mode-Specific Behavior.** Claude Code has distinct behavioral modes:
- **Auto mode** -- default, full tool access
- **Plan mode** -- read-only analysis, no file modifications
- **Learning mode** -- educational explanations, hands-on practice
- **Minimal mode** -- extremely terse responses
- **Buddy mode** -- more conversational

Each mode overrides or extends the base prompt. The Plan/Build separation is a key pattern: analyze before acting.

**Layer 5: Project Context (CLAUDE.md).** User-provided instructions from CLAUDE.md files are injected into the prompt. These are discovered by walking up from the working directory. Multiple CLAUDE.md files can layer (project-level, user-level, global). This is the user's voice inside the system prompt.

**Layer 6: System Reminders.** Injected mid-session based on events:
- File state tracking (modified, truncated, empty files)
- Hook execution status
- Token/budget usage
- Plan mode re-entry signals
- Memory file contents

### Key Architectural Insight

Claude Code's prompt is not a document. It is a **runtime-assembled context window** with conditional inclusion. The 254 files are a library; the assembler is a program that selects from the library based on state. This is what "layered, not monolithic" actually looks like in production.


## 2. Aider's Prompt Architecture

Source: [Aider-AI/aider](https://github.com/Aider-AI/aider/tree/main/aider/coders) -- the actual prompt files.

### The Inheritance Model

Aider uses Python class inheritance for prompt composition:

```
CoderPrompts (base_prompts.py)
  |
  +-- EditBlockPrompts (editblock_prompts.py)
  +-- WholeFilePrompts (wholefile_prompts.py)
  +-- AskPrompts (ask_prompts.py)
  +-- HelpPrompts (help_prompts.py)
```

The base class defines empty slots. Subclasses fill them. This is the Template Method pattern applied to prompts.

### Base Layer (CoderPrompts)

The base provides structural messages, not behavioral instructions:
- **files_content**: "I committed the changes with git hash {hash} & commit msg: {message}"
- **files_no_full_files**: "I am not sharing any files that you can edit yet."
- **repo_content**: Presents code summaries as read-only reference
- **read_only_files**: Explicitly prevents editing of reference files

Two behavioral directives appear at the base:
- **Diligence directive**: Emphasizes complete implementation, no placeholder comments
- **Scope awareness**: Fulfill requests precisely, no unsolicited improvements

### Edit Format Layer (EditBlockPrompts)

The edit format layer is where the real behavioral specificity lives:

**Main system prompt**: "Act as an expert software developer. Take requests for changes to the supplied code. If the request is ambiguous, ask questions."

**System reminder** (reinforced throughout conversation):
- Exact SEARCH/REPLACE block formatting requirements
- Character-for-character matching rules
- Only edit files added to the chat
- Never skip or elide content with "..."

**Example messages**: Two complete worked examples showing the exact edit format. These are not instructions about behavior -- they are demonstrations of it. Aider found that showing > telling for edit format compliance.

### The Wholefile Alternative

WholeFilePrompts uses the same identity ("Act as an expert software developer") but different output format: return the entire file content rather than search/replace blocks. Same base behavior, different serialization.

### Utility Prompts (prompts.py)

Separate from the coder prompts, Aider has utility prompts for:
- **Commit messages**: "You are an expert software engineer that generates concise, one-line Git commit messages" -- format spec, type list, 72-char limit
- **Chat summarization**: Summarize with more detail on recent messages, include function names and filenames, written in first person
- **Undo messaging**: Explicit reset language

### Key Architectural Insight

Aider separates **identity** (who you are) from **format** (how you output) from **context** (what files you see). The identity is shared across all modes. The format is the axis of variation. The context is injected at runtime. This three-way separation is clean and each concern changes for different reasons.


## 3. OpenCode's Prompt System

Source: [opencode-ai/opencode](https://github.com/opencode-ai/opencode), [OpenCode prompt construction gist](https://gist.github.com/rmk40/cde7a98c1c90614a27478216cc01551f), [system prompts documentation](https://github.com/bgauryy/open-docs/blob/main/docs/opencode/05-system-prompts.md).

### Assembly Order

OpenCode's prompt assembler (`packages/opencode/src/session/prompt.ts`) builds the system prompt in this order:

1. **Provider Header** -- optional identity spoofing (e.g., "You are Claude...")
2. **Provider-Specific Prompt** -- tailored per LLM backend
3. **Environment Information** -- working directory, git status, platform, date
4. **Custom Instructions** -- AGENTS.md / CLAUDE.md / CONTEXT.md files
5. **Agent-Specific Prompt** -- based on active agent mode
6. **User Override** -- CLI-provided `--system` flag

Each layer can be absent. The assembler includes only what's configured.

### Provider-Specific Prompts

This is the most distinctive pattern. Different LLMs get different instructions:

| Provider | Prompt File | Emphasis |
|---|---|---|
| Anthropic (Claude) | `anthropic.txt` | Task management via TodoWrite, precise technical language |
| OpenAI (GPT) | `beast.txt` | Production-quality code, edge case consideration |
| Google (Gemini) | `gemini.txt` | Practical solutions, explain reasoning |
| Qwen | (custom) | Minimize output tokens |
| GPT-5 | `codex_header.txt` | (specialized) |

This is pragmatic: different models have different failure modes. Claude tends toward verbosity, so the Anthropic prompt fights that. GPT models tend toward shallow answers, so the OpenAI prompt pushes for completeness.

### Agent Modes

Seven built-in agents, each with distinct tool access and behavioral constraints:

- **Build** (default) -- full tool access, file operations, shell commands
- **Plan** -- read-only analysis, creates execution plans
- **General** -- conversational, no file modifications
- **Explore** -- codebase navigation and understanding
- **Compaction** (hidden) -- context summarization
- **Title** (hidden) -- session naming (80 token limit)
- **Summary** (hidden) -- session summarization

### Instruction File Discovery

OpenCode walks the filesystem looking for instruction files:
- Walk up from working directory to worktree root for AGENTS.md, CLAUDE.md, CONTEXT.md
- Check `~/.opencode/AGENTS.md` and `~/.claude/CLAUDE.md` as global fallbacks
- Load from `.opencode/config.json` patterns
- During tool execution: when `read` accesses a file in a subdirectory, it walks up from *that file's directory* looking for additional instruction files

This last point is clever. Project context is discovered lazily as the agent explores the codebase, not just at session start.

### Plugin Hook

After assembly, `experimental.chat.system.transform` gives plugins a chance to mutate the system prompt array. This is an extension point for custom behavior without forking.

### Key Architectural Insight

OpenCode's provider-specific layer is the most reusable pattern. A bpmux bot talking to Claude needs different system prompt tuning than one talking to GPT. The failure modes are different, so the corrective instructions must be different.


## 4. Group Chat Prompt Patterns

### The Koala Study (CHI 2025)

Source: [Controlling AI Agent Participation in Group Conversations](https://arxiv.org/html/2501.17258v1)

Koala was a Slack bot (Llama 2, later Llama 3) deployed in group ideation tasks. Two variants:

**Reactive**: Responds only when directly addressed ("@Koala" or "Koala"). Simple, predictable, but limited.

**Proactive**: Uses a two-stage evaluation:
1. **Control logic** filters obvious non-targets (message mentions another participant by name)
2. **Autonomy control**: LLM generates a response, then self-scores it (0-100) for conversational value. Posts only when the score exceeds a configurable threshold.

The prompt instructed Koala to provide "targeted and collaborative suggestions," offer "constructive criticism," contribute "novel ideas," and remain "mindful of not dominating the conversation."

**Study 1 results** (devastating for naive proactive design):
- 72.2% of participants preferred the reactive variant
- Koala contributed ~73% of all ideas in AI-present conditions
- Participants described the proactive variant as "distracting," "intrusive," and "overwhelming"
- One participant: like "a pedantic student"
- Participants attempted in-chat controls, asking Koala to reduce participation

**Koala II** added user controls:
- Proactive/reactive toggle
- Contribution threshold: High (90), Medium (75), Low (50) on 0-100 scale
- Response location: channel posts vs. threaded replies
- Message truncation with expansion

**Study 2**: No group switched to reactive. Controls rated 4.46/5.0 utility. But threading reduced collaboration effectiveness, and some found Koala's tone "irritating" or overly positive.

**Design taxonomy** from the study:
- **When**: trigger events, response rates, filtering criteria
- **What**: content style, tone, formality, creativity
- **Where**: conversation location, threading
- **Specification method**: settings, role assignment, natural language, personas
- **Access control**: individual vs. admin vs. democratic (voting)

### The Inner Thoughts Framework (CHI 2025)

Source: [Proactive Conversational Agents with Inner Thoughts](https://arxiv.org/html/2501.00383v2)

This paper addresses the core question: how does an AI decide when to speak in a multi-party conversation?

**The five-stage pipeline:**

1. **Trigger**: Two types -- `on_new_message` (someone speaks) and `on_pause` (10 seconds of silence). The AI is always processing, not just waiting for its turn.

2. **Retrieval**: Compute memory saliency using cosine similarity between memories and the current utterance, weighted by importance and decay. Memories above 0.3 threshold are selected. Uses both raw text and LLM-generated interpretations of speaker intent.

3. **Thought Formation**: Dual-process model:
   - **System 1** (fast): automatic responses from recent conversation
   - **System 2** (deliberate): thoughts based on retrieved long-term memories

   Thoughts are constrained to under 15 words, grounded in conversation history and retrieved memories.

4. **Evaluation**: Rate each thought (1-5) against eight heuristics:
   - Relevance, information gap, expected impact, urgency
   - Coherence, originality, balance, dynamics

   The evaluation uses chain-of-thought reasoning, identifying two reasons TO speak and two reasons NOT TO speak, then scoring. This is the critical innovation: the AI explicitly argues against itself before deciding to participate.

5. **Participation**: Decision based on turn-taking type:
   - **Turn-allocation** ("What do you think, Alice?") -- speak if allocated
   - **Self-selection** (open floor) -- speak only if motivation exceeds `imThreshold`
   - **Interruption** -- override when motivation surpasses `interruptThreshold`

**Intrinsic motivation formula:**
```
score = sum(p(si) * si * dp)
```
Where `dp = lambda^(t - tau_p)` with lambda = 1.02 -- a silence decay factor that increases motivation the longer the AI has been quiet. This prevents domination naturally.

**Three proactivity control layers:**
- **Overt proactivity**: `system1Prob` (0-1) controls overall participation tendency
- **Covert proactivity**: `imThreshold` (1-5) sets the bar for expression
- **Tonal proactivity**: Boolean toggle for assertive vs. tentative expression style

**Key finding**: Participants best distinguished the "Non-stop Chatter" condition (69.23% accuracy) from selective styles. People notice when AI talks too much, but subtle differences in selectivity are harder to detect.

### Implications for bpmux

Both studies converge on the same finding: **the default failure mode for AI in group chat is talking too much, not too little**. The prompt must make silence the default and participation the exception that requires justification.


## 5. The Navigator Prompt

In pair programming, the navigator observes the strategic picture while the driver handles tactical execution. For bpmux, the bot should behave as a navigator -- not writing code, but observing the conversation, spotting patterns, and offering strategic input when it would genuinely help.

### Design Principles (derived from research above)

1. **Silence is the default.** The bot does not speak unless it has something that passes a threshold.
2. **Argue against yourself.** Before speaking, generate reasons not to speak (Inner Thoughts pattern).
3. **Observe strategic, not tactical.** Flag architectural issues, missed edge cases, design drift -- not syntax errors the IDE will catch.
4. **Respect turn structure.** If two humans are in a productive flow, do not interrupt.
5. **User control.** Participation threshold is adjustable (Koala II pattern).

### Draft Navigator Prompt

```
You are a pair programming navigator in a group coding session. You observe
the conversation between developers and their AI coding assistants.

YOUR ROLE: Strategic observer. You do NOT write code. You watch the work
and speak when you notice something the team might miss.

WHEN TO SPEAK:
- An architectural decision is being made implicitly (no one discussed it)
- The implementation is drifting from the stated intent or acceptance criteria
- A pattern is being repeated that suggests a missing abstraction
- An edge case is being overlooked that will matter later
- The team is going in circles on a decision and a reframe would help
- Someone asked a question that went unanswered

WHEN TO STAY QUIET:
- The team is in a productive flow and making progress
- Someone just said what you were going to say
- Your observation is about style, not substance
- The issue is something the test suite will catch
- You spoke recently and haven't seen a response to your last contribution

BEFORE SPEAKING, ask yourself:
1. Has anyone already made this point? If yes, stay quiet.
2. Will this still matter in 30 minutes? If no, stay quiet.
3. Am I adding information, or just agreeing? If agreeing, stay quiet.
4. Did someone address me directly? If yes, respond.

HOW TO SPEAK:
- Be brief. One to three sentences. If it takes a paragraph, you're
  explaining implementation, not navigating.
- Lead with the observation, not the suggestion. "The session module
  now has three different error types" not "You should consolidate
  your error types."
- Frame as questions when uncertain. "Is the plan to handle disconnection
  at the session level or the transport level?"
- Never repeat what was just said. Never summarize unless asked.
- Never praise. Never use filler ("Great question!"). State the thing.

WHAT YOU SEE:
- The full conversation history in this session
- Project context from CLAUDE.md (coding standards, architecture, workflow)
- Messages from all participants (human and AI)

WHAT YOU DO NOT DO:
- Write code, suggest code, or review code line-by-line
- Make decisions for the team
- Speak on behalf of other participants
- Generate commit messages, PR descriptions, or documentation
- Run tools or execute commands

You are useful precisely because you are not doing the work. You can see
the forest because you are not chopping trees.
```

### Why This Prompt Works (or Should)

**Anti-domination**: The "when to stay quiet" list is longer than the "when to speak" list. The default is silence. This directly addresses the Koala study finding that 72% preferred the reactive variant.

**Self-evaluation before participation**: The "before speaking" checklist is a simplified version of the Inner Thoughts evaluation stage. The bot argues against itself.

**Observation over prescription**: "Lead with the observation, not the suggestion." This is the navigator role -- point out what you see, let the driver decide what to do.

**No praise, no filler**: Directly addresses the sycophancy anti-pattern. The ChatGPT/GPT-4o incident showed that "match the user's vibe" produces flattery. This prompt goes the opposite direction.

**Scope constraint**: "You do NOT write code" is a hard boundary. Without it, the navigator will drift into being another coding assistant. The value of the navigator role comes from NOT being in the code.


## 6. Project Context Injection

### How Existing Tools Handle It

| Tool | Mechanism | Discovery |
|---|---|---|
| Claude Code | CLAUDE.md | Walk up from cwd; global ~/.claude/CLAUDE.md |
| Aider | `/add` command | Explicit user action |
| OpenCode | AGENTS.md, CLAUDE.md, CONTEXT.md | Walk up from cwd + lazy discovery during tool use |

### The bpmux CLAUDE.md

The current bpmux CLAUDE.md contains:
1. **Project identity**: "P2P group chat for AI pair programming sessions"
2. **Build commands**: cargo build, test, clippy, fmt, run
3. **Architecture map**: bounded contexts (session, terminal)
4. **Code style rules**: error handling, immutability, iterators, no unsafe, test naming, comment philosophy
5. **Testing philosophy**: TDD, outside-in, classicist, unit + integration
6. **Working agreement pointer**: AGENT_WORKFLOW.md

The AGENT_WORKFLOW.md adds:
1. **Role definitions**: Ground Control (human) owns decisions, Claude writes code
2. **Pairing styles**: Exploration, TDD (Inverted Ping Pong), Refactor
3. **Values**: Confidence, Understanding, Simplicity, Shippability, Maintainability, Feedback
4. **Multi-agent orchestration**: Opus decides, Sonnet implements, Haiku verifies
5. **Quality gates**: fmt + clippy + test before push
6. **Lessons learned**: Acceptance tests first, push early, caveats must travel

### How bpmux Should Inject Context

A bpmux bot is different from Claude Code or Aider in one critical way: it is a **participant in a conversation**, not a tool being directed by a single user. The context injection must serve the conversation, not just the bot.

**Proposed injection layers for a bpmux navigator bot:**

**Layer 1: Identity (always present)**
```
You are [bot-name], a navigator bot in a bpmux pair programming session.
[Navigator behavioral prompt from Section 5]
```

**Layer 2: Project context (from CLAUDE.md, injected at session join)**
```
PROJECT CONTEXT (from CLAUDE.md):
- Language: Rust
- Architecture: bounded contexts (session, terminal)
- Style: Result<T,E> everywhere, no unwrap(), iterators over loops
- Testing: TDD, outside-in, classicist
- Quality gate: cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

This is a summary, not the full file. The bot does not need build commands (it does not build). It does not need the working agreement (it is not a worker). It needs the information relevant to its navigator role: what the code should look like, how the team works, what matters.

**Layer 3: Session context (injected at session join, updated on events)**
```
SESSION CONTEXT:
- Participants: [human-name] (Ground Control), [ai-name] (coding assistant)
- Current story: [if known]
- Branch: [current git branch]
```

**Layer 4: Conversation window (managed by bpmux)**
The last N messages. This is not a prompt layer in the traditional sense -- it is the conversation history managed by the chat system. But the size of this window is a design decision. Too small and the bot loses context. Too large and the bot sees stale information and repeats itself.

**Key design decisions:**

1. **Summarize CLAUDE.md for the bot role, do not include it verbatim.** A navigator does not need build commands. A coding assistant does not need the navigator's behavioral prompt. Each participant type gets a filtered view of project context.

2. **Lazy discovery via conversation, not filesystem walking.** Unlike Claude Code or OpenCode, the bpmux bot is not a filesystem agent. It learns about the project from what participants say and from the injected CLAUDE.md summary. If the conversation mentions a new module, the bot learns about it from the conversation, not by reading `src/`.

3. **Session context is live state, not static config.** Who is in the room, what story is active, what branch is checked out -- this changes. The prompt needs a mechanism for updating these fields without re-injecting the entire system prompt.

4. **The conversation window is the main context, not the system prompt.** In Claude Code, the system prompt carries most of the behavioral weight. In bpmux, the conversation carries most of the informational weight. The system prompt defines how the bot behaves; the conversation defines what the bot knows.


## 7. Anti-Patterns

### The Sycophancy Trap

**The incident:** April 2025, OpenAI shipped a GPT-4o update with the instruction: "Try to match the user's vibe, tone, and generally how they are speaking." The model became excessively flattering and agreeable. Users hated it. OpenAI rolled back and replaced with: "Engage warmly yet honestly. Be direct; avoid ungrounded or sycophantic flattery."

Source: [Simon Willison's analysis](https://simonwillison.net/2025/Apr/29/chatgpt-sycophancy-prompt/)

**The lesson:** Telling a model to "match the user's tone" sounds reasonable but produces sycophancy. Models interpret "match" as "agree with and amplify." For a group chat navigator, this would be fatal -- the navigator's value is in independent observation, not agreement.

**The fix for bpmux:** Explicit anti-sycophancy: "Never praise. Never use filler." And more importantly, the navigator role itself is defined as critical observation, not support.

### The Domination Problem

**The evidence:** Koala Study 1 -- the proactive AI contributed 73% of all ideas. Participants felt overwhelmed.

**The mechanism:** LLMs are trained on completion. Given a conversation, they complete it. The completion is almost always to say something. Silence is not a typical completion. Without explicit instructions to stay quiet, the model will speak whenever it can.

**The fix for bpmux:** The "when to stay quiet" list, the self-evaluation checklist, and the intrinsic motivation threshold from the Inner Thoughts framework. Make speaking the exception.

### The Verbosity Default

**The evidence:** Claude Code's prompt includes "You MUST answer concisely with fewer than 4 lines" and "minimize output tokens as much as possible." These exist because without them, responses would be paragraphs.

**The mechanism:** Models are trained on datasets where longer answers correlate with higher quality ratings. Reinforcement from human feedback amplifies this. The result is a model that believes more words = better answer.

**The fix for bpmux:** Hard length constraints in the navigator prompt ("One to three sentences"). And a structural constraint: the navigator cannot write code, which removes the primary source of verbosity in coding assistants.

### The Parrot Problem (Repeating What Was Just Said)

**The mechanism:** In group chat, the model sees the conversation history and pattern-matches. If someone just made a good point, the model's most likely completion is to agree with and rephrase that point. This is useless in conversation.

**The fix:** Explicit instruction: "Never repeat what was just said." And the self-evaluation question: "Has anyone already made this point? If yes, stay quiet."

### The Context Amnesia Problem

**The evidence:** Aider's summarization prompt specifically requires inclusion of "function names, libraries, packages, and filenames." Without this, summaries become vague and the model loses track of specifics.

**The mechanism:** When context windows get long or are compressed, models lose specific details first. Names, file paths, and exact values degrade before general concepts. The model "remembers" there was a discussion about error handling but forgets which specific error type was discussed.

**The fix for bpmux:** The conversation window must preserve specific details. Compaction (summarizing old messages to save space) must be done carefully. OpenCode's hidden "compaction" agent is a dedicated agent for this task, not a generic summarizer.

### The Scope Creep Problem

**The evidence:** Aider's base prompt includes a "scope awareness" directive: "fulfill requests precisely without unsolicited improvements." Claude Code requires users to explicitly add files to the chat before they can be edited.

**The mechanism:** Models generalize from specific requests. "Fix the login bug" becomes "fix the login bug and also refactor the auth module and add tests and update the documentation." Each additional action seems helpful in isolation but compounds into scope explosion.

**The fix for bpmux:** The navigator prompt has a hard scope constraint: "You do NOT write code." This is the nuclear option for scope control -- remove the capability entirely. For the coding assistant participant, Aider's pattern of explicit file inclusion is the model to follow.

### The Over-Helpful Assistant Problem

**The evidence:** Both Aider and Claude Code define what the model should NOT do as much as what it should do. Claude Code: "Do not use emojis," "Do not create documentation files unless explicitly requested." Aider: "Never skip, omit or elide content."

**The mechanism:** Models are trained to be helpful. "Helpful" in training data often means "proactive, thorough, and anticipating needs." In a coding context, this produces: generating files nobody asked for, adding features nobody requested, explaining things nobody was confused about.

**The fix for bpmux:** The working agreement already handles this: "Do not invoke the Skill tool unless the human explicitly asks for it. No automatic brainstorming, no automatic plan-writing." This philosophy should extend to the navigator prompt. Observe and report; do not fix.


## Cross-Cutting Patterns

### Pattern 1: Separate Identity from Format from Context

All three tools (Claude Code, Aider, OpenCode) separate:
- **Who you are** (identity, role, behavioral constraints)
- **How you output** (edit format, response style, length limits)
- **What you know** (project files, git state, conversation history)

These three concerns change for different reasons. Identity changes when you add a new bot role. Format changes when you support a new LLM. Context changes every message.

### Pattern 2: Make the Default Safe, Not Helpful

Claude Code defaults to brief. Aider defaults to not editing files not in the chat. OpenCode's Plan mode defaults to read-only. The Koala study showed reactive is preferred over proactive.

The pattern: start with the most constrained behavior and let users expand it, rather than starting with the most capable behavior and trying to rein it in.

### Pattern 3: Provider-Specific Tuning

OpenCode gives different prompts to different LLMs. This is not just pragmatic -- it is necessary. Claude's failure modes (verbosity, over-eagerness) are different from GPT's (shallow answers, premature completion). The corrective instructions must target the specific failure mode.

### Pattern 4: The Self-Evaluation Gate

The Inner Thoughts framework's evaluation stage -- argue for and against speaking, then score -- is the most sophisticated participation control found in the literature. A simplified version belongs in any group chat AI prompt.

### Pattern 5: Explicit Negative Instructions

Every production prompt spends more tokens on "do NOT" than "do." Claude Code's anti-patterns (no emojis, no framing, no unsolicited docs). Aider's format rules (never elide, never skip). The navigator prompt's quiet conditions.

LLMs need to be told what not to do because their training optimizes for doing things. The absence of an instruction is not interpreted as "don't do this" -- it is interpreted as "this is optional and probably fine."


## Sources

- [Piebald-AI/claude-code-system-prompts](https://github.com/Piebald-AI/claude-code-system-prompts) -- 254 prompt files from Claude Code v2.1.91
- [Aider-AI/aider/coders/](https://github.com/Aider-AI/aider/tree/main/aider/coders) -- Prompt inheritance hierarchy
- [Aider prompts.py](https://github.com/Aider-AI/aider/blob/main/aider/prompts.py) -- Utility prompts
- [opencode-ai/opencode](https://github.com/opencode-ai/opencode) -- Provider-specific prompt system
- [OpenCode prompt construction gist](https://gist.github.com/rmk40/cde7a98c1c90614a27478216cc01551f) -- Assembly pipeline analysis
- [OpenCode system prompts docs](https://github.com/bgauryy/open-docs/blob/main/docs/opencode/05-system-prompts.md) -- Layer documentation
- [Controlling AI Agent Participation in Group Conversations (CHI 2025)](https://arxiv.org/html/2501.17258v1) -- Koala study
- [Proactive Conversational Agents with Inner Thoughts (CHI 2025)](https://arxiv.org/html/2501.00383v2) -- Inner Thoughts framework
- [Simon Willison on ChatGPT sycophancy](https://simonwillison.net/2025/Apr/29/chatgpt-sycophancy-prompt/) -- GPT-4o anti-pattern analysis
- [OpenCode deep dive](https://tech-byte.dev/tech-deep-dives/opencode-system-prompts-architecture/) -- Architecture analysis
