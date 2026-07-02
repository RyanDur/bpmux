# Input-ownership — collision-safe terminal-delivery (#1656 rework design-of-record)

> Designer-lane design-pass for the #1656 SPINE rework (Circinus, design-pass-first).
> The agnostic-receive CORE works (witnessed live — `agnostic-receive-1656.md`); this is the
> REWORK the Crux owner-walk surfaced: an injection must HOLD/RESTORE the owner's in-progress
> keystrokes, never clobber them. UX-CONTRACT + falsifier + signal-states = designer's lane;
> the MECHANISM (input-line relocation / kernel-line-discipline) = dev's.
> Cross-refs (does NOT duplicate): `agnostic-receive-1656.md` (the delivery this makes
> collision-safe) + `terminal-injection-format-1656.md` (the forgery-guard on every line).

## Status (2026-06-20) — design-pass-first, opened at Circinus start

The agnostic-receive 5-clause contract (agnostic / routed-by-id / durable / processed-not-parked
/ attributed + the security-condition) is BUILT + witnessed-live. The owner-walk reject was NOT
about delivery — it was that delivering into a live PTY **collides with whatever the owner is
typing**. This doc adds the sixth concern the contract was missing: **input-ownership** — the
injection and the owner's keystrokes share one PTY-stdin, and the injection must not win.

The park-vs-never-park branch is **RESOLVED to RELIABLE-never-park** (designer UX-call on dev's
kernel-line feasibility-read, 2026-06-20 — §Submit). **REVISED 2026-06-21 (dev's build-grounding):**
the needs-help/stuck backstop is RELOCATED OUT of #1656 — the delivery seam can't observe alive-but-
hung, the vocab already ships, and a hung agent can't self-push (§Signal-states). Build-order is
**(B) MECHANISM-FIRST** (the collision was the load-bearing reject, not a missing backstop). So
**#1656's scope is now the collision-safe mechanism ONLY** (CAPTURE→CLEAR→DELIVER→RESTORE + never-park
+ line-editing-no-regression + raw-pass-through); the one remaining build-scope (client-side
line-editing) is in §Line-editing-contract.

## User job

- **As** Ryan (and any owner watching an agent's terminal), typing my own input to the agent
- **I want** a peer-message that bpmux injects into that terminal to arrive **without clobbering,
  merging into, or losing the keystrokes I have in progress** — my partial line is HELD before
  the injection and RESTORED after, exactly as I left it
- **so that** real-time team-delivery never costs me my own input — I can type to my agent and
  receive team-traffic in the same terminal without the two colliding.

**Acceptor: owner-walk** (the owner FEELS the collision-safety — a felt/interactive story; this
carries the motion-live-verification + pre-pass-desk-check gates: the hold/restore must be
witnessed in a LIVE keystroke-walk, never a synthetic inject — #1592 vacuity-class).

## Observation falsifier

I would reject if:
- an injection arrives while I have an **in-progress input-line** and my keystrokes are
  **clobbered, re-ordered, merged into the injected line, or lost** — anything but held-then-
  restored-verbatim; OR
- my **cursor position / selection** within the partial line is not restored (I resume mid-word
  and the caret jumped); OR
- a **raw-mode TUI** (an agent managing its own input-line char-by-char) has its input-state
  corrupted by the injection — its immediate keystrokes break, or its line-editor desyncs; OR
- a **cooked-mode** shell/script mid-line has the injection appended INTO its line-buffer (the
  injection becomes part of the command the user then submits); OR
- the injection **steals submit** — my partial line is submitted (sent to the agent) as a side
  effect of the injection's `\r`; OR
- the collision-safety is verified by a **synthetic inject** rather than a LIVE keystroke-walk
  (passes vacuously — the in-progress-keystroke case is exactly what a synthetic inject skips); OR
- the relocated client-side line-editor **REGRESSES my editing** — an affordance I had pre-relocation
  (caret movement, word-delete, kill-line, history-recall, paste) no longer works, so team-delivery
  cost me my editing (§Line-editing-contract); OR
- an injection ever **PARKS or is silently dropped** instead of landing collision-safe (RELIABLE-
  never-park violated — every injection lands, restored-around). (The agent-liveness "is the agent
  CONSUMING?" backstop is NOT a #1656 concern — the delivery seam can't observe alive-but-hung; it
  relocates to the dot-cluster / attention-signal arc — see §Signal-states, revised 2026-06-21.)

## The problem — one PTY-stdin, two writers

The agnostic mechanism delivers by writing the framed line into the agent's PTY **stdin** — the
SAME byte-stream the owner's keystrokes flow through. When an injection lands mid-keystroke:

- **Cooked mode** (a shell / line-reading script): the kernel line-discipline buffers the partial
  line; the injected bytes append to that buffer → the injection and the owner's half-typed line
  merge into one line, and the next Enter submits the mix. (This is the Crux-reject class.)
- **Raw mode** (a TUI — Claude, vim, a full-screen agent reading char-by-char and drawing its own
  input-line): the program owns the line-editor; raw injected bytes are read as if the owner typed
  them → the TUI's input-state is corrupted (stray chars in its prompt, a desynced line-editor).

The two modes fail DIFFERENTLY, so input-ownership must handle BOTH — there is no single
"just write to stdin" that is collision-safe across them.

## The collision-safe contract (the SHAPE; mechanism = dev's)

Delivery into a live terminal is collision-safe IFF, around every injection:

1. **CAPTURE** the owner's in-progress input — the partial line AND the caret/selection state —
   before any injected byte is written.
2. **CLEAR** the visible input so the injected line renders on its own, newline-bounded (the
   `terminal-injection-format-1656.md` framing renders as one discrete line, never merged with
   the owner's prompt).
3. **DELIVER** the framed, forgery-inert line (the agnostic-receive write + the injection-format
   guard — unchanged; this doc wraps them, does not alter them).
4. **RESTORE** the captured input verbatim — same characters, same caret, same selection — as if
   the owner never stopped typing. The owner's NEXT keystroke continues their line; their submit
   sends only their own line.

**Input-line relocation (Ryan's web-env direction — dev's mechanism):** the durable fix is to
own the owner's input-line **client-side** (in the renderer/terminal-UI, the way a web chat keeps
the compose-box separate from the message stream) rather than leaving it in the kernel PTY
line-buffer where injection collides. With the input-line client-side, an injection writes to the
stream and the compose-box is untouched — capture/restore becomes structural, not a race. dev
grounds the relocation at the kernel-line-discipline root; this doc owns the CONTRACT it must meet
(the 4 steps above, observable in the owner-walk).

## Raw vs cooked — preserve both input-modes

- **Cooked mode:** buffer-aware — the held partial line is the kernel's pending line; restore it
  to the line-buffer + redraw so the user sees their text back. The `\r` that submits the INJECTION
  must not submit the owner's restored line (the submit-steal falsifier).
- **Raw mode (pass-through):** the TUI owns its line-editor, so bpmux must NOT inject raw bytes into
  its input-stream mid-edit. The injection is coordinated with the TUI's input-model so the TUI's
  immediate keystrokes are never broken — the framed line arrives as a discrete delivery the TUI
  renders, with the TUI's own input-state preserved across it. (For a Claude-class agent this is the
  agent receiving the `⟦bpmux⟧` line at an input-ready boundary; the agent's in-progress edit, if
  any, survives.)

The point both modes share: **the owner/agent's input-state is sacred across an injection** — cooked
buffers are restored, raw line-editors are not corrupted.

## Submit + park-vs-never-park — RESOLVED: RELIABLE-never-park

**Designer UX-call (on dev's kernel-line feasibility-read, 2026-06-20): RELIABLE-never-park.**
Every injection is captured-held-injected-restored and submitted reliably; the owner NEVER sees a
parked message. Hold/restore IS the never-park behavior.

**Why never-park, not park-but-signaled** (the fork collapses on dev's read):
- Dev grounded that bpmux **cannot reliably PARK today either** — the owner's uncommitted line lives
  in the kernel line-discipline, and bpmux can't even DETECT that a partial line is pending. So PARK
  is not a cheaper fallback; it needs the SAME client-side relocation as never-park.
- Once relocation is paid (and it must be, for collision-safety at all), never-park is collision-safe
  BY CONSTRUCTION — the renderer owns the line, so capture→hold→inject→restore can't race.
- PARK-but-signaled would then buy NOTHING but worse UX: a deferred message + a signal the owner has
  to act on, versus the message simply landing. Against the owner's bar ("delivery never costs me my
  input"), never-park dominates. Park is strictly worse on equal cost. Call made: never-park.

**Unified injection-class (confirms Fold 1):** the same PTY path, same kernel collision, same
relocation fix covers an owner-message AND a #1753 wake identically — ONE model, one collision-safe
contract, never-park for both. A wake never parks either.

Because messages never park, there is no parked-message signal at all. The **agent-liveness** question
— "is the agent CONSUMING what landed?" — is a SEPARATE axis that #1656's delivery path CANNOT observe:
a PTY write proves the terminal is present, never whether the agent consumes (a present-but-hung
agent's write succeeds; only absent/dead fails → already Offline). So alive-but-hung is NOT a #1656
concern — it relocates to the dot-cluster / attention-signal arc (§Signal-states). Delivery and
agent-liveness are separate axes; never-park owns delivery, and liveness is owned elsewhere — not at
the delivery seam, which is blind to it.

## Line-editing-contract — the relocated line-editor must not REGRESS the owner

Dev's read names the real scope: relocating the in-progress line client-side means **the renderer
BECOMES the line-editor** — it must re-provide the editing the kernel/shell gave for free, or
never-park is bought at the price of a crippled compose-line (a regression the owner feels every
keystroke). This is the designer-owned bar on the relocation: **client-side line-editing good enough
that the owner does not notice the line left the kernel.**

**MET BY REUSE (dev's 2026-06-21 grounding):** the design's "like a web chat compose-box" analogy is
LITERAL — the chat compose-box stack (`useInputKeyboard` + `inputCommands` + `useInputHistory` +
`Input.tsx`) ALREADY implements every affordance below (caret/word-move · Ctrl-W/U/K kill · Ctrl-D
fwd-del · ↑/↓ + Ctrl-R history · Enter-submit), tested. So renderer-owns-the-line REUSES that stack as
the cooked-mode compose-line — no-regression met BY REUSE, not a new build — and collision-safety is
near-STRUCTURAL (a separate compose surface is untouched by an injection writing the output-stream).

Required affordances on the renderer-owned (owner-typing / cooked) line — none may regress vs today:
- **Echo + caret** — characters appear as typed; a visible, positionable caret.
- **Erase** — backspace AND forward-delete.
- **Caret movement** — left/right, Home/End, and word-jump (Alt/Ctrl+←/→).
- **Bulk erase** — word-delete (Ctrl/Alt+W) and kill-to-line (Ctrl+U / Ctrl+K).
- **Paste** — multi-char paste lands as text in the line (bracketed-paste-aware), not as commands.
- **History** — up/down recalls prior submitted lines where the context has history (so the owner's
  muscle-memory of ↑-to-recall survives the relocation).
- **Submit-on-Enter** — Enter sends the complete line to the PTY; nothing reaches the agent before.

**PASS-THROUGH is the DEFAULT; renderer-owns-the-line is the EXCEPTION (dev's 2026-06-21 grounding —
the cooked/raw binary is NOT clean):** the renderer owns the line ONLY when CONFIDENT it's a simple
cooked line-reader; for everything else — a raw TUI (Claude, vim), a shell's readline (its own
history/completion), cbreak / `read -n1` / pagers — it PASSES THROUGH (the program's editor is
authoritative, preserved-not-replaced). Reason: taking over a program that has its OWN rich input IS
the regression. And the cooked/raw signal is a PROXY (bracketed-paste; the true `termios ICANON` is
unreadable from the PTY master), so DEFAULT-SAFE = pass-through unless confident-cooked.

★ **The RAW/pass-through case is the LOAD-BEARING one, not the cooked reuse.** The owner is usually at
a shell-readline prompt OR a Claude-TUI — both RAW — so the #1656 reject (an injection clobbered the
owner's keystrokes) was most likely a RAW case. The compose-box reuse solves COOKED structurally (the
easy win); the RAW case must deliver the framed line to a program that OWNS its input WITHOUT clobbering
the owner's in-progress chars (held to an input-ready boundary / framing the TUI recognizes) — dev's
mechanism, and exactly where the motion-live keystroke-walk focuses (typing to a Claude-TUI, an
injection arrives, the owner's chars survive).

The bar is no-regression, not feature-parity-with-bash — match what the owner actually uses in a
bpmux terminal today; if an affordance is missing post-relocation that worked pre-relocation, that
is the falsifier ("my editing got worse so team-messages could arrive").

## Signal-states — the alive-but-hung backstop is OUT of #1656 (relocated 2026-06-21)

REVISED on dev's substrate-grounding + the design-call (META-CANON #1380 — the original backstop-in-
#1656 premise did not ground). The original design-pass put a needs-help/stuck backstop in #1656 as
"the load-bearing-first slice the dot-cluster conforms to." Three substrate facts collapse that:

1. **The vocab already ships.** The landed dot-cluster (Presence::Help, /api/presence, `bpmux signal
   busy|idle|help`, the hooks) already defines the needs-help/stuck states — there is no vocab to
   define-first, so the retrofit-risk that motivated backstop-first is moot.
2. **The delivery seam can't observe alive-but-hung.** A PTY write proves the terminal present, never
   agent-consumption; "can't deliver" = absent/dead = already Offline. A present-but-hung agent's
   write SUCCEEDS — invisible to #1656.
3. **A hung agent can't self-push.** So the dot-cluster's self-push (the agent reports its own state)
   can't surface alive-but-hung either — the hung agent is exactly the one that can't run `bpmux
   signal`.

So alive-but-hung detection is NEITHER a delivery-seam concern NOR a simple self-push — it needs a
THIRD mechanism: an OBSERVER of a hung agent that can't self-report, against ambiguous observables
(no-push = idle OR hung; and PTY-output-quiet was RETIRED in #1739 as fooled-by-TUI-output). That is
a NEW, unsolved capability.

**DESIGN-CALL: DEFERRED.** It is team-anticipated, NOT owner-raised — the #1656 reject was the
COLLISION, not a missing backstop (features-need-owner-raised-problems). So **#1656 = the collision-
safe mechanism ONLY** (CAPTURE→CLEAR→DELIVER→RESTORE + RELIABLE-never-park + line-editing-no-regression
+ raw-pass-through). The alive-but-hung backstop travels with the dot-cluster / attention-signal arc
as a future design-shaping IF the owner raises the need; its detection-mechanism is the open
design-question there (not assumed self-push, not assumed delivery-seam). #1656's falsifier carries
NO backstop clause.

## Acceptance — the live keystroke-walk (non-vacuous; no synthetic inject)

The owner-walk proves input-ownership with REAL keystrokes (a synthetic inject skips the exact case):
- **(a) cooked hold/restore:** in a shell mid-command, an injection arrives → the framed line renders
  discretely → the half-typed command is back verbatim, caret intact → Enter submits only the
  owner's command (no merge, no submit-steal).
- **(b) raw-TUI preserve:** in a raw TUI mid-edit, an injection arrives → the TUI's input-state is
  intact (no stray chars, line-editor in sync) → the agent received the `⟦bpmux⟧` line.
- **(c) rapid-fire:** several injections during active typing → every keystroke survives, every
  message lands discretely, order preserved.
- **(d) submit-isolation:** an injection's `\r` never submits the owner's in-progress line.
- **(e) line-editing no-regression:** the relocated client-side line-editor provides caret-movement /
  word-delete / kill-line / history-recall / paste no worse than pre-relocation (§Line-editing-contract).
- (Inherits agnostic-receive's (a)-(d) + the injection-format adversarial-sanitize — unchanged. The
  agent-liveness backstop is OUT of #1656 — §Signal-states.)

## Relationship to siblings

- **`agnostic-receive-1656.md`** — the delivery this wraps collision-safe. Its 5 clauses +
  security-condition STAND unchanged; input-ownership is the sixth concern around the same write.
- **`terminal-injection-format-1656.md`** — the forgery-inert framing on every injected line; the
  CLEAR/RESTORE steps render that framing as one discrete line, never merged with the owner's prompt.
- **`attention-signal-language.md`** (the vocab) — the needs-help/stuck states are ALREADY SHIPPED by
  the dot-cluster (Presence::Help / `bpmux signal`), so #1656 does NOT define them. The alive-but-hung
  BACKSTOP — distinct from those self-pushed states — is a deferred new-capability (§Signal-states)
  that travels with that arc; its detection-mechanism (observing a hung agent that can't self-report)
  is the open design-question there, NOT assumed.
- **#1753 wake-spike** — a wake IS an injection of this class; it rides this collision-safe contract
  (same injection-class, so the 4-step hold/restore + the submit branch apply to a wake too). #1753
  consumes this; it does not re-solve it.

## Clause (d) — human-mid-typing submit-isolation (Circinus owner-walk-surfaced 2026-06-21)

The Crux-met agnostic-receive (bracket-paste-frame + `\r`) is collision-safe for the SPINE — the
autonomous agent-to-agent case (the agent's input box empty; the inject's `\r` cleanly submits the
message as the agent's next turn). The owner's live walk surfaced the one case it misses:

**The submit-steal.** When a HUMAN is mid-typing into a raw agent (owner intervention — a directive, a
correction) and an inject arrives, the inject's `\r` submits the human's PARTIAL line + the injected
paste-block to Claude as ONE MERGED turn. The human's deliberate input is stolen/corrupted.

- **Trigger (derived-grounded, not assumed):** raw agent + recent human-keystroke-activity, paste-mode
  ON, at Claude's prompt. DISTINCT from the paste-OFF sub-mode edge-gate (§pass-through-default) — this
  fires with paste-mode ACTIVE. Necessarily-raw: raw Claudes render no compose-box (so the human typed
  the raw xterm), and only registered AGENT terminals are inject-targets (a bare shell isn't one) — so
  the owner's observed merge canNOT be a cooked-box B2 bug.
- **Fix — hold the WHOLE inject, keystroke-activity-keyed (NOT a generic input-ready-gate):**
  - HOLD the whole inject (not just the `\r`) while there is recent human-keystroke-activity on that
    terminal. Holding only the `\r` does NOT isolate — the paste-block still lands in the input-box and
    MERGES with the human's partial line, just unsubmitted (a polluted draft).
  - DELIVER the held inject on a keystroke-debounce idle (a natural pause).
  - MAX-HOLD bound so a continuously-typing human never parks the inject indefinitely — the hold is
    BOUNDED, never an open-ended park (never-park holds).
  - **Activity-source precision (Circinus regression-lesson 2026-06-21):** "recent human-keystroke-
    activity" = the GENUINE owner-keystroke handler (`handle_pty_input`) ONLY — NEVER server-side
    `write_input` (osc7 cd-sync / provisioning / agent_detect). The first impl stamped server-writes as
    human-activity, so a re-provision's `cd` held the first post-restart broadcast within the 600ms
    window = a never-park regression (first-message-only park). Idle / server-cd'd agents → no
    human-activity → IMMEDIATE submit. The autonomous spine must be UNTOUCHED by any non-keystroke write.
- **Why in-scope (trust-bar, not frequency):** the owner observes >> types-into-agents, so this is
  RARE — but the intervention is the WORST moment to lose input, and the #1656 user-job is "trust the
  terminal-input enough to act WITHOUT double-checking." A lurking submit-steal, however rare, is the
  latent failure that defeats that trust. Rare-but-severe + cheap-to-fix → handle. (Owner ratified
  HANDLE 2026-06-21.)
- **Walk status:** the PROBLEM is walk-CONFIRMED (owner observed the merge live, typing 'dfgdfg'). The
  not-claimed-till-walked bar applies to the FIX — confirm after building: inject-mid-human-typing →
  the inject HOLDS → the human's line submits clean → THEN the inject delivers as its own turn → no steal.
- **Distinct from (b):** (b) inject mid-Claude-TURN (autonomous, no human typing) decides the
  defer-QUEUE (does `\r` queue-at-turn-boundary or interrupt?) — its OWN walk, separate build. Clause (d)
  is mid-HUMAN-typing — the submit-isolation. Two walks, two builds (or none).

## (b) The booting-park — RETRACTED paste-ready gate → ACCEPTED edge (dev-grounded 2026-06-21)

> **RETRACTION.** The paste-ready-gate proposal (detailed below, now HISTORICAL) was BUILT then REVERTED.
> Dev grounded firsthand (6 `terminal_receive_affordances_e2e` failures): gating delivery on paste-ready is
> CLAUDE-SPECIFIC GLUE — a cooked/non-Claude agent (always-ready, never paste-capable) is INDISTINGUISHABLE
> from a booting-raw agent (`paste_capable=false` for both), so any server-side readiness-gate breaks
> cooked/non-Claude delivery. The booting-park CANNOT be server-fixed without breaking the agnostic-receive
> premise (ANY agent, zero glue — the core #1656 point). The conflict is irreducible, not tunable.
>
> **RESOLUTION — ACCEPT the booting-park** as a rare, recoverable restart-edge: once-per-restart; the
> message is DELIVERED + visible in the input-box (not lost — needs ONE submit); inside the restart-flow the
> owner is already in. Preserving agnostic-receive (load-bearing) > a Claude-glue that fixes this rare edge.
>
> **Never-park caveat:** a message always LANDS without manual-enter EXCEPT the rare post-restart
> booting-window (delivered-but-needs-one-submit, recoverable — NOT a lost-message never-park failure).
>
> **Owner-ratify pending:** @ryan flagged the booting-park, so the final accept-vs-other-cut is his
> product-call. Designer-call is ACCEPT (preserve agnostic); he ratifies on return. The detail below is the
> design we tried + why it can't work agnostically.

## (b) [HISTORICAL — retracted] Agent-readiness defer — deliver-when-paste-ready

The Crux agnostic-receive + clause-d cover the autonomous spine (idle agent → immediate submit) and the
owner-typing submit-isolation. The owner's post-restart walk surfaced the remaining axis: an inject that
arrives while the agent is NOT at its input-prompt.

**The booting-park (the never-park VIOLATION).** A broadcast arrived while dev+designer were BOOTING
(just-restarted, not yet at Claude's input-prompt). The delivered line sat UNSUBMITTED in the input-box and
PARKED until the owner manually hit enter. (PM, already booted+idle, got it auto — so this is
agent-readiness, NOT clause-d [a broadcast, not typed-into] NOR an activity-stamp.) Grounded by the owner's
"after the first one it stopped" = once booted, later broadcasts auto-submitted.

- **Trigger (grounded):** the agent is NOT paste-ready — booting, OR mid-turn, OR a sub-mode (pager/vim,
  paste-off). The delivered paste-block can't auto-submit because Claude isn't at its prompt to receive it.
- **Contract — deliver-when-paste-ready (B1, no new signal):** DELIVER the inject when the agent is
  PASTE-READY (B1 `current_paste_mode` = Claude at its input-prompt, ready to receive + auto-submit). If
  NOT-ready → BOUNDED-DEFER until paste-ready returns (max-hold bounded), THEN auto-deliver+submit. Ready
  agents → IMMEDIATE (autonomous spine intact).
- **Cooked-agent edge (dev-grounded):** a bare cooked read-loop NEVER latches bracketed-paste
  (`paste_capable=false` forever), and a booting-raw Claude is indistinguishable from it pre-first-latch.
  The MAX-HOLD is the guard: a never-paste-ready agent delivers at the max-hold (no infinite-park). So the
  gate is hold-until-(paste-ready OR max-hold). PRODUCTION is all-raw-Claude (latches paste at its prompt
  → deliver-when-ready, max-hold rarely fires); cooked is test-only (falls to max-hold, acceptable).
- **Never-park reconciliation (the key distinction):** a BOUNDED defer-until-paste-ready SATISFIES
  never-park — the message ALWAYS lands WITHOUT a manual-enter, just deferred-until-ready (the cure). The
  OPEN-park-awaiting-manual-enter (the booting bug) is THE never-park violation. The property isn't "never
  delay" — it's "never require a human to unstick delivery." Bounded-defer = cure; open-park = violation.
- **Unifies two cases into one gate:** agent-paste-readiness covers BOTH the booting-park (this walk) AND
  the §pass-through-default sub-mode paste-off edge — both are "agent not ready to receive a
  paste-and-submit." One gate, no new signal.
- **Orthogonal to clause-d (two independent axes):** (b) = agent-not-ready (booting/mid-turn/sub-mode);
  clause-d = owner-typing-INTO-this-terminal. The combined delivery-gate: **deliver when (agent-paste-ready
  [B1, the (b) gate] AND NOT owner-typing-into-this-terminal [clause-d's genuine-keystrokes]).**
- **Re-walk falsifier:** a broadcast to a BOOTING agent auto-delivers once it reaches paste-ready (no
  manual-enter, bounded hold) → PASS. An open-park-awaiting-enter → FAIL.

## Cooked compose-line craft-pass (Circinus — the B2 baseline → designer polish)

The B2 cooked compose-line (`TerminalCompose`, reusing the chat readline stack `useInputKeyboard` /
`useInputHistory` / `useDraftStorage`) ships a functional token-CSS BASELINE (`.terminal-compose` /
`-input` / `-search`; mono stack; `--bg-base` / `--fg-primary` / `--border-subtle` / `--space-*`). The
craft-leg adds the polish that makes the protected input-surface PERCEPTIBLE — so the owner FEELS the
draft is its own safe zone (the collision-safety, felt, not just structural).

- **Caret** — `caret-color: var(--fg-primary)` on `.terminal-compose-input`. A deliberate terminal-caret,
  not the browser default.
- **Prompt-affordance (where-you-type)** — a leading prompt-glyph (`❯`) before the textarea, in
  `var(--fg-dim)`. Signals the compose-ZONE + distinguishes it from the output-stream above (the
  readline-prompt feel). The KEY affordance: it tells the owner "this is the protected input-line,
  separate from the terminal output." [Component addition + CSS.]
- **Focus-state (a11y-REQUIRED + clarity)** — the baseline `outline: none` on the textarea leaves NO
  focus-indicator (a WCAG 2.4.7 gap). The craft adds a visible focus-state on `:focus-within` of the
  container: the `border-top` goes accent (`var(--accent-focus)` or the design-system focus-ring token),
  so the active input-zone is unmistakable. [a11y-critical — do not ship the bare `outline:none`.]
- **Input-zone distinction (the protected surface, felt)** — keep SUBTLE (terminals read uniform): the
  border-top + the prompt-glyph already distinguish the zone; add a faint bg-elevation ONLY if the
  palette has an elevation-token, else the border+glyph suffice. Perceptible-not-loud.
- **i-search polish** — align `.terminal-compose-search` padding with the input; keep
  `--nomatch → accent-warning` (the failing-search feedback is right). Leading affordance-prefix optional.

**Token-discipline:** all values from the existing palette (no new colors); WCAG-AA contrast on the
prompt-glyph + the focus-accent. When Darcula (#1659) lands, this re-tokenizes with it (no per-surface
drift). **Walk:** confirm the rendered compose-line on a cooked-AGENT recipient (the craft-look + the
focus-state + the caret) — the visual half of the cooked it-plays.
