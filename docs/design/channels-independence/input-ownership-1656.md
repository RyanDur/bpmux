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
kernel-line feasibility-read, 2026-06-20 — §Submit). The whole model is now settled; the one
remaining build-scope it surfaces (client-side line-editing) is specified in §Line-editing-contract.

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
- an injection that **cannot** be delivered collision-safely (the agent isn't input-ready) is
  silently dropped OR the agent is hung with **no needs-help/stuck signal** (§Signal-states).

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

The needs-help/stuck signal (§Signal-states) is therefore NOT a parked-message signal (messages never
park) — it is the **agent-liveness** backstop: the message always LANDS collision-safe, but if the
AGENT is hung / not consuming, the owner is signalled to go look. Delivery and agent-liveness are
separate axes; never-park owns delivery, the backstop owns liveness.

## Line-editing-contract — the relocated line-editor must not REGRESS the owner

Dev's read names the real scope: relocating the in-progress line client-side means **the renderer
BECOMES the line-editor** — it must re-provide the editing the kernel/shell gave for free, or
never-park is bought at the price of a crippled compose-line (a regression the owner feels every
keystroke). This is the designer-owned bar on the relocation: **client-side line-editing good enough
that the owner does not notice the line left the kernel.**

Required affordances on the renderer-owned (owner-typing / cooked) line — none may regress vs today:
- **Echo + caret** — characters appear as typed; a visible, positionable caret.
- **Erase** — backspace AND forward-delete.
- **Caret movement** — left/right, Home/End, and word-jump (Alt/Ctrl+←/→).
- **Bulk erase** — word-delete (Ctrl/Alt+W) and kill-to-line (Ctrl+U / Ctrl+K).
- **Paste** — multi-char paste lands as text in the line (bracketed-paste-aware), not as commands.
- **History** — up/down recalls prior submitted lines where the context has history (so the owner's
  muscle-memory of ↑-to-recall survives the relocation).
- **Submit-on-Enter** — Enter sends the complete line to the PTY; nothing reaches the agent before.

**Raw-mode TUIs are the exception, by design:** when a raw TUI (Claude, vim) owns its own line-editor,
the renderer does NOT replace it — keystrokes pass through and the TUI's editor is authoritative; the
collision-safety there is the §Raw-mode pass-through-preserve, not the client-side editor. The
client-side line-editor governs the cooked/owner-typing case (where the kernel-buffer collision lived);
the raw case is preserved, not relocated. (How the renderer distinguishes the two + switches is dev's
mechanism.)

The bar is no-regression, not feature-parity-with-bash — match what the owner actually uses in a
bpmux terminal today; if an affordance is missing post-relocation that worked pre-relocation, that
is the falsifier ("my editing got worse so team-messages could arrive").

## Signal-states (the needs-help/stuck backstop — load-bearing-first slice of the attention-signal vocab)

When an injection cannot complete collision-safely (PARK-but-SIGNALED, or the agent is stuck and not
consuming input), the backstop is a **needs-help/stuck** signal — the owner sees that this agent has
an undelivered message / is not input-ready, trustworthy enough to ACT on (go look) without
double-checking.

These two states — **running** (input-ready, delivery flows) and **stuck/needs-help** (not
input-ready, an injection would park / a help is pending) — are the **load-bearing-first slice** of
the one **attention-signal design-language** (forthcoming sibling doc, authored whole: running/stuck
+ needs-help + dormant-park [#1753] + owner-attention). Dev builds the needs-help backstop FIRST as
the shared seam (the dot-cluster #1747/#1748/#1750/#1721 + this backstop conform to it), so the
vocab's needs-help/stuck states are defined HERE and the full system slots onto them without
retrofit. Pin (matches the trustworthy-dot acceptance-bar): a stuck/needs-help signal **CLEARS when
the condition resolves** (the message lands / the help is answered); an input-ready agent never reads
stuck.

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
- **(e) stuck-backstop:** an agent not input-ready → the injection does NOT corrupt/drop silently →
  the needs-help/stuck signal raises, and CLEARS when delivery completes.
- (Inherits agnostic-receive's (a)-(d) + the injection-format adversarial-sanitize — unchanged.)

## Relationship to siblings

- **`agnostic-receive-1656.md`** — the delivery this wraps collision-safe. Its 5 clauses +
  security-condition STAND unchanged; input-ownership is the sixth concern around the same write.
- **`terminal-injection-format-1656.md`** — the forgery-inert framing on every injected line; the
  CLEAR/RESTORE steps render that framing as one discrete line, never merged with the owner's prompt.
- **The attention-signal design-language** (forthcoming sibling) — the full signal vocab; the
  needs-help/stuck states defined here are its load-bearing-first slice (dev's shared-seam build
  conforms to them). #1753's dormant-park + owner-attention slot onto the same system.
- **#1753 wake-spike** — a wake IS an injection of this class; it rides this collision-safe contract
  (same injection-class, so the 4-step hold/restore + the submit branch apply to a wake too). #1753
  consumes this; it does not re-solve it.
