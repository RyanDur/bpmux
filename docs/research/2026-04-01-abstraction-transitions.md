# The Abstraction Transition: Historical Parallels to AI-Assisted Development

*What happened when high-level languages replaced assembly — and what it tells us about now.*

---

## The Pattern

Every major abstraction transition in computing history follows the same arc:

1. **A new layer of abstraction emerges** that automates what was previously done by hand
2. **Practitioners resist**, citing loss of control, performance concerns, and threats to professional identity
3. **The abstraction wins** because it enables things that were previously impossible
4. **The scope of what gets built expands** — practitioners don't write less, they build more
5. **The practitioners who adapted became more valuable**, not less

This pattern repeated with assembly → high-level languages, procedural → object-oriented, manual deployment → CI/CD, and is now repeating with manual coding → AI-assisted development.

---

## The FORTRAN Transition (1957)

### The Priesthood

John Backus, creator of FORTRAN, described the culture of early programming as a "priesthood" — programmers regarded themselves as guardians of arcane knowledge, possessing skills far too complex for ordinary mortals. They wanted simple mechanical aids for clerical drudgery but "regarded with hostility and derision more ambitious plans to make programming accessible to a larger population."

### The Resistance

Assembly programmers opposed FORTRAN on three grounds:

**Performance:** Compiled code couldn't match hand-optimized assembly. This was initially true — early compilers did produce suboptimal machine language. Contemporary accounts describe skilled assembly programmers outperforming early compiler output (see Backus's own accounts of FORTRAN's early reception).

**Control:** Surrendering intimate control over memory and CPU instructions to a compiler felt risky. "If I don't code it down to the metal, how can I trust what's happening?"

**Identity:** Making programming easier might reduce the prestige of the seasoned programmer. High-level languages opened the door for newcomers to write code without years of experience in binary or assembly.

### The Outcome

Richard Hamming observed: "The opposition to FORTRAN and any automatic coding system seems to me very very high. And the courage [Backus] had to persist I think should be recognized."

FORTRAN won. Compilers improved dramatically over subsequent decades. What was formerly a laborious task of manual instruction-level programming could now be expressed at a much higher level of abstraction (IBM's historical accounts describe order-of-magnitude reductions in program length). The programmers who resisted weren't replaced — the demand for programmers grew as the scope of what software could do expanded. Some assembly experts moved into compiler development and systems programming, though the claim that resisters specifically became compiler writers is a commonly told narrative, not a documented historical finding.

And critically: **programmers didn't write less code. They wrote orders of magnitude more, tackling problems that would have been economically impossible before.**

---

## The Efficiency Paradox

Addy Osmani articulated this pattern for the AI era:

> "Every time we've made it easier to write software, we've ended up writing exponentially more of it. When high-level languages replaced assembly, programmers didn't write less code — they wrote orders of magnitude more, tackling problems that would have been economically impossible before."

This is the Jevons Paradox applied to software: making a resource more efficient to use doesn't reduce demand — it increases it. The constraint shifts from "can we build this?" to "should we build this?" Imagination and judgment replace implementation capacity as the limiting factors.

---

## The Emotional Arc

What's striking about the FORTRAN transition is that the emotional arc is identical to today:

**Then:** "I spent years mastering the PDP-11 instruction set. I can write tighter code than any compiler. Why would I give up that control?"

**Now:** "I spent years mastering software architecture. I can design better systems than any AI. Why would I give up that judgment?"

The answer, both times, is: **you don't give it up. You apply it at a higher level.**

The assembly programmer's understanding of memory and CPU didn't become worthless — it became the foundation for writing better compilers, understanding performance bottlenecks, and debugging what the abstraction layer got wrong. The programmer who understood the machine at the lowest level was the most valuable person in the room when things went wrong.

The same is true now. The developer who understands architecture, testing, security, and system design doesn't become worthless when AI writes the code — they become the most valuable person in the room because they know when the AI is wrong, and why.

---

## The Stigmergic View of Abstraction Transitions

We observe a structural similarity between abstraction transitions and stigmergy: the compiler modifies the environment (produces machine code), and that modification enables the next level of work. This is our interpretive lens, not a claim made by the historical sources. No one in the FORTRAN era described what they were doing as stigmergy (Grassé coined the term for termite behavior, not computing).

That said, the structural parallel is useful for design thinking: if AI-assisted development is an abstraction transition where developers leave traces (intent, tests, decisions) and AI responds by modifying the shared artifact, then bpmux's role is to make those traces visible to all participants. This framing informs our design — it doesn't prove it's correct.

## What This Means for bpmux

### Our ideas need to become bigger

If every abstraction transition expands the scope of what gets built, then AI-assisted development will expand the scope of what development teams tackle. Stories that were "too big" become feasible. Projects that were "too ambitious" become practical. The thin slices get thicker — not because we abandon incremental delivery, but because each increment delivers more value.

A story might move from feature-level ("add pagination to the user list") to application-level ("build the user management system"). Not because we skip the small steps, but because the AI handles the steps that used to consume our time, freeing us to think about bigger problems.

### The working agreement will evolve

Every abstraction transition changed the relationship between the practitioner and their tools. Assembly programmers had to learn to trust compilers. Today's developers are learning to trust AI. The Ground Control / Major Tom working agreement reflects where we are now — the human owns all decisions, the AI writes code, trust is earned incrementally.

As models become more reliable and capable, the agreement will shift. More autonomy for the AI. More delegation. Bigger scopes of independent work. The human moves from reviewing every line to reviewing outcomes. Not because the human becomes less important, but because the unit of review changes — just as code review shifted from individual instructions to functions to modules to pull requests.

### The change should make aspirations grow, not drive people apart

The FORTRAN transition made programming accessible to more people. It didn't destroy the community of programmers — it expanded it. The assembly experts became compiler writers, language designers, and systems architects. The newcomers, freed from instruction-level details, tackled problems that had never been automated before.

The same can happen now. AI-assisted development doesn't need to be a story of replacement. It can be a story of amplification — where the people who understand how to build software well become the ones who teach AI to build software well, and the expanded capability allows everyone to tackle bigger, more meaningful problems.

bpmux is built on this belief: that the right tools don't replace collaboration, they amplify it. Two developers with AI assistants shouldn't be competing for the same work — they should be tackling problems that would have been impossible for two developers alone.

---

## The Oscillation

There is a natural oscillation between remorse and excitement at every abstraction transition. The remorse is for skills being commoditized. The excitement is for possibilities being unlocked. Both are real. Both are valid.

John Backus felt it. The assembly programmers felt it. We feel it now.

The resolution, every time, has been the same: the people who leaned into the new abstraction — while carrying forward the deep understanding from the old one — ended up in a better place. Not because the transition was painless, but because the expanded scope of what was possible made the craft more interesting, not less.

---

## Sources

- [When Compilers Were the 'AI' That Scared Programmers](https://vivekhaldar.com/articles/when-compilers-were-the--ai--that-scared-programmers/)
- [From Assembly to AI: Programming Abstractions](https://www.jamesluterek.com/blog/ai-next-abstraction-layer/)
- [The Efficiency Paradox — Addy Osmani](https://addyosmani.com/blog/the-efficiency-paradox/)
- [How John Backus' FORTRAN Beat the Machine Code 'Priesthood'](https://thenewstack.io/how-john-backus-fortran-beat-machine-codes-priesthood/)
- [Higher Level Languages — Computer History Museum](https://www.computerhistory.org/revolution/early-computer-companies/5/117)
- [IBM: John Backus](https://www.ibm.com/history/john-backus)
