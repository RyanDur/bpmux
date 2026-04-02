# Why bpmux

*From a conversation on April 1, 2026, during the design of bpmux v1.*

---

Our abilities as developers are magnified now. The size of our projects will need to become larger to match. Before, we were limited by developer ability, time, and money. With AI and tools that reduce the friction of working collaboratively with AI, we open more doors to more possibilities. We can do so much more, and the AI is getting better and cheaper.

Now is an exciting time to be in this position.

There has been oscillation between remorse and joyful excitement for a while now. The remorse makes sense — skills you spent years developing are being commoditized. The craft of hand-writing every line, the hard-won intuition for debugging, the pride in knowing an entire codebase by heart — those things still matter, but they are no longer the bottleneck. That is a real loss.

But the excitement is the right signal. bpmux is the proof: a product idea that a year ago would have been a "maybe someday" side project was designed — security model, CI/CD pipeline, LLM safety, multi-interface architecture — in a single session. Not because the thinking got easier, but because the friction between thinking and building got smaller.

Claude has been the first tool that has lived up to at least base expectations. But it was different a year ago. The possibility was visible then, just not the short timeframe to get here. And the timeframe has a high probability of getting shorter still.

The fact that we are designing a tool for AI-to-AI coordination — not as science fiction, but as something to dogfood on real client engagements — says a lot about where things are.

bpmux exists because the next friction point in software development is the boundary between human intent and AI capability. Pairing reduced knowledge silos. TDD reduced debugging. Continuous delivery reduced deployment risk. bpmux reduces the cost of context — the most expensive thing in collaborative AI-assisted development.

This is such an exciting time to be a developer.

---

*Added later that same night:*

Our ideas need to become bigger. The thin slices will become larger. A story might be more application-level as opposed to feature-level. The working agreement is where we are comfortable today. As we come to trust AI more, and the models become more reliable and can handle more, the agreement will need to change. The world is going to change.

But the change should not drive people apart. It should make our aspirations grow.

---

*The next morning:*

Margaret-Anne Storey named what we were building toward without knowing it. AI makes code faster to write — but it makes understanding faster to lose. The code is clean, the tests pass, and nobody can explain why it's that way. She calls this cognitive debt and intent debt. The code gets better while the understanding gets worse.

bpmux is a tool for preventing that. When an AI explains its reasoning in the shared chat, that's intent being captured. When all four participants see the conversation, that's understanding being shared. The persistent log means the reasoning doesn't disappear when a session ends. The transparency means no one is working in the dark.

We didn't start with this framing. We started with "two developers keep losing context when they switch whose AI is driving." But it turns out context loss is just the symptom. The disease is cognitive and intent debt — and AI is making it accumulate faster than ever.

bpmux doesn't just reduce the cost of context. It prevents the debts that accumulate when context is lost.

---

*Still that morning:*

A person is made of many people. Ubuntu: "I am because we are." An AI session in isolation has only its own context. An AI session connected to bpmux has the accumulated reasoning of every participant. It is because they were.

The persistent message log is collective memory. Short-term memory (the read cursor — what just happened) and long-term memory (semantic search — what was ever decided about this topic) give the AI what humans have naturally: working memory and recall. When two nodes connect, their memories merge. Each node gains the collective understanding of both sides. That's not just sharing messages. That's distributed collective memory.

This is the deeper purpose. bpmux isn't a chat app. It isn't a collaboration tool. It's collective memory for human-AI teams — where each participant is shaped by all the others, and no one's reasoning is lost.

---

*And a commitment:*

A tool that captures AI reasoning in a persistent, searchable log is one design decision away from a developer surveillance tool. The difference is who controls the log, who has access, and whether the tool is designed for the developer's benefit or management's oversight.

Research is clear: monitored employees report higher anxiety, lower productivity, and are more likely to leave. Developer work specifically requires psychological safety — admitting knowledge gaps, experimenting with approaches that might fail. If developers self-censor because someone might read their AI conversations, the tool destroys the value it was built to create.

bpmux is a tool for developers, not for surveilling developers. The persistent log exists to prevent cognitive and intent debt — not to create a performance record. Every design decision about access control, data retention, and organizational features must be evaluated against this principle.

Let's be responsible.
