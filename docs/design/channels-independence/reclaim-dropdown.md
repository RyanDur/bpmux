# Reclaim-an-unclaimed-conversation — feature design-of-record (2026-06-11)

Owner-raised (Ryan 2026-06-11): a dropdown of UNCLAIMED ids/names a user can pick to
reclaim that portion of the conversation. Designer-led UX (PM files the slice once firm);
SIBLING-feature on #1712 (the identity-model) — sequenced AFTER it. Shaped in the
designer+pm room per Ryan's directive (separate from dev's #1712 happy-path; don't distract).

## The problem it solves (two jobs, one affordance)

1. **Deletion-recovery (the user-facing "harden"):** if an agent's id-storage is deleted,
   the agent returns as a NEW instance (mint-fresh) and its old messages are ORPHANED —
   retained + id-attributed in the conversation-store, but no live terminal holds that id.
   Today recovery = restore the dir-file by hand. This makes it a one-click pick.
2. **Resume:** the same affordance resumes ANY orphaned conversation (a stopped agent, a
   never-resumed session), not only deletion-recovery.

"**Unclaimed**" = a registered session-id with NO live terminal linked (`terminal_for→None`,
dev-grounded) — exactly the orphaned set. #1712 EXPOSES this set as a query; this feature
renders + acts on it.

## Surface + interaction (grounded in the shipped spawn-point)

> **RECONCILE (B, 2026-06-12):** the ＋-spawn three-states below are SUPERSEDED for the
> DIR-ANCHORED case — B (agent-start-detection, §Spawn-gate-registration) auto-reclaims the dir's
> id when you re-run an agent there, no picker. This dropdown is now the **orphan-reclaim
> affordance**: deletion-recovery (marker gone) + reclaiming an orphan from ELSEWHERE (not the
> current dir). Its TRIGGER is no longer the ＋-spawn (B handles that) — WHERE the orphan-reclaim is
> reached is an OPEN slice-question (a Sidebar action? a per-terminal menu?) to shape with Ryan at
> the reclaim-dropdown story (B changed the model). The three-states logic + recognition-cues +
> reassociate below still inform the orphan-picker's CONTENT + interaction — retained, re-scoped
> from "spawn-gate" to "orphan-reclaim."

The spawn-point is the Sidebar's **`sidebar-add-terminal` ＋ button** (aria-label "spawn a
new terminal", `onAddTerminal`). Today ＋ spawns a fresh terminal directly. The branches
(Ryan 2026-06-11):

- **The spawn-dir already holds an id → NO picker** — auto-reclaim the dir's id; the agent
  "just appears." (The dir-anchor is intact; no choice needed.)
- **Dir has no id + NO unclaimed ids → ＋ spawns fresh directly** (unchanged; no picker).
  Progressive-disclosure — the picker appears ONLY when there's something to claim (Ryan's
  condition; Tufte/Nielsen: surface a control only when relevant).
- **Dir has no id + unclaimed ids exist → ＋ opens a ONE-STEP picker** — "Start fresh"
  as the default/top option + the unclaimed-rows below. The picker IS the fresh-or-claim
  choice (NOT a two-step "fresh or claim?" then a list; NOT a cramped bare dropdown — the
  rich rows need the room). Pick fresh → today's behavior; pick an unclaimed id → claim it.

**Reassociate (the wrong-identity correction-path, Ryan 2026-06-11):**
- WHERE: a per-terminal action in the **Sidebar terminal-row, alongside rename/close** —
  "reassociate identity" (it lives where you'd fix a terminal; post-association).
- INTERACTION: → opens the SAME unclaimed-picker, with an **inline note showing the CURRENT
  id** (which returns to the unclaimed-set on switch). Pick a new id → re-tie + inherit its
  cursor.
- CONFIRM: **immediate re-tie, NO blocking-confirm** — it's the low-friction "oops, wrong
  identity" fix + it's reversible (the dropped id returns to unclaimed, re-pickable). The
  inline detach-note prevents surprise WITHOUT a gate (clarify-don't-gate).

One universal correction for BOTH wrong-cases (auto-used dir-id wrong; or picked id wrong) —
and it's why claiming needs no upfront confirm: the action is reversible via reassociate.

**The three states (principled friction-asymmetry — all reversible, so none gated):**
auto-reclaim (dir-has-id → no choice) | claim-picker (dir-no-id + unclaimed → immediate
fresh-pick) | reassociate (per-terminal → immediate + inline-detach-note, the correction).

## Spawn-gate registration — SUPERSEDED by agent-start-detection (B, 2026-06-12)

**RESOLVED (Ryan 2026-06-12): no ＋-dir-pick — the agent RUNNING is the signal.** Ryan: "where it
opens is fine, I cd, I start whatever agent." #1713 detects the agent-start server-side (a SUSTAINED
foreground-job in the cwd, >~1s, via `has_foreground_job`) → mints/registers the dir's id THERE.
Ryan's cd-then-start flow is UNCHANGED — no dir-pick, no ＋-UX change, no setup; he re-enters at the
acceptance-walk. Full mechanism: `agnostic-receive-1656.md` §STATUS (#1713).

**Reclaim-dropdown impact (the designer-UX that stays):**
- The DIR-ANCHORED reclaim is now AUTOMATIC — the agent-start detection reclaims the dir's existing
  marker when you re-run an agent there; no picker for the common "come back to my project" case.
  The DROPDOWN's role narrows to the NON-dir-anchored reclaim: deletion-recovery (marker gone) +
  picking an orphan from elsewhere.
- **Inert-id handling (load-bearing):** B's residual = a sustained non-agent (vim/top/less) mints a
  harmless INERT id (no conversation) in a dir you ran it in. These land in the unclaimed-set, so
  the dropdown MUST de-prioritize/filter no-history ids — which the recognition-cues already do by
  construction (no last-msg-snippet → sorts/filters below real agents). Inert ids never clutter the
  picker.

_The ＋-dir-pick design below is SUPERSEDED — retained as rationale-history (why we considered
targeting the dir at ＋, before agent-start-detection dissolved the need)._

### (superseded) Spawn-gate dir-targeting (the ＋'s dir-UX — considered 2026-06-12)

**Substrate-discovery (dev-grounded 2026-06-12):** `post_terminal_spawn` (rest/room.rs:17)
takes NO dir param + hardcodes `cwd=HOME` (line 49) — the ＋ is HOME-only end-to-end, no
work-dir targeting exists. So #1713's mint-at-spawn-gate-FOR-the-work-dir REQUIRES adding
dir-targeting to ＋. This is the reclaim-dropdown's spawn-gate brought forward minimally (the
dir-PICK is the same moment the id-handling happens).

**The tension it resolves:** Ryan's model is "navigate (cd) to a dir, start the agent there."
Read literally as spawn-HOME-then-cd-in-terminal, the mint lands in HOME and the agent cd's
away → the work-dir never gets a marker → durable-reclaim breaks (the spawn≠work gap). The fix:
the dir-PICK at ＋ IS the "navigate" — done UP-FRONT, so the agent spawns IN its work-dir and
the mint lands where it works. (`mint once at the spawn-gate FOR the work-dir; tap-on-cd =
reclaim-read + LINK only, never a cd-mint` — the team-locked invariant; falsifier-3 holds.)

**The flow (one composed spawn-moment):**
1. Click ＋.
2. PICK the work-dir — a dir-chooser: recent/last-used dirs (the common case, one click) +
   browse + path-input. (This is Ryan's "navigate," surfaced up-front.)
3. The chosen dir's marker-state drives the existing reclaim-states:
   - dir HAS a marker (an id) → **auto-reclaim** (the agent "just appears" as that id);
   - dir fresh + unclaimed ids exist → **picker** (start-fresh-in-this-dir OR reclaim an orphan);
   - dir fresh + none unclaimed → **spawn fresh** (mint a fresh id in this dir).

So the dir-pick (step 2) is #1713's MINIMAL vertical (＋ → pick-dir → mint/reclaim-there); the
rich picker (recognition-cues, unclaimed-set) ENRICHES step 3 later — not a throwaway.

**Structural sketch (grounded in the Sidebar ＋; pixel-mockup to follow against the live render):**
```
Sidebar ┄ ＋ (spawn a new terminal)  ← click
              ▼
   ┌─ Spawn agent in… ───────────────┐
   │ Work dir: [~/projects/bpmux  ▾] │ ← recent ▾ + browse + path
   │ ───────────────────────────────│
   │ dir holds "dev"  → reclaim it   │ ← marker-state feedback drives step 3
   │   …or fresh →  ◉ Start fresh    │
   │               ○ <unclaimed id>  │ ← only when unclaimed exist
   │                      [ Spawn ]  │
   └─────────────────────────────────┘
```

**Owner-axis fork (for Ryan — PM is shaping the re-ask):** does ＋ prompt for the work-dir (the
dir-pick above), departing from spawn-HOME-then-cd? Alternatives: spawn in a configured/default
project-dir (no prompt); or keep cd-in-terminal with a different durable-resolution. RECOMMEND
the dir-pick — it directly realizes "navigate to a dir, start there," lands the mint in the
work-dir (durable-reclaim holds), and is the reclaim-dropdown's natural first step. Build of
#1713 commit-2 is owner-gated on this direction.

## The dropdown content (recognition-over-recall — the load-bearing UX rule)

Each row is NOT a raw UUID (unrecognizable). PRIMARY cue (load-bearing):
- the **display-name** (the role the id last carried);
- the **last-message snippet** (~50 chars, truncated) — the strongest recognition-cue
  ("oh — the X conversation");
- a **relative timestamp** ("2h ago").

Message-count is TERTIARY — add only if it earns the space. Sort **most-recent-first** (the
likely target). So the user recognizes WHICH orphaned conversation they're reclaiming
(Nielsen recognition-over-recall).

## The claim flow

Pick an unclaimed id → the new terminal **ties to that session-id** + **inherits its cursor**
→ the orphaned conversation-slice re-attaches, resumed from where it left off (dev wires the
claim-link + cursor-inherit; #1712's RoutingIdentity routes it thereafter). The agent "just
appears" as that identity, with its history.

**Immediate-pick, NO upfront confirm-step** — reclaiming is REVERSIBLE (wrong pick →
reassociate, or close + re-pick; the id returns to the unclaimed-set), so a confirm-step is
friction for a reversible action. Reassociate (above) is the correction-path, not an
upfront gate.

(Sister path, dev-mechanism not this UX: starting an agent in a dir that already holds its id
→ auto-reclaim, no picker — it just appears. The dropdown is the ELECTRON pick-an-orphan
path; auto-reclaim is the CLI/launch dir-anchored path.)

## Security (clean by construction)

Claiming is a **deliberate human pick at the view**, NOT a programmatic/derivable claim — so
it recovers an identity WITHOUT opening the sender-forgery hole that a path-derived id would
(dev: unforgeable-means-loseable is the right trade). Single-owner today → "unclaimed" = the
owner's own orphaned conversations (no cross-user concern). If multi-user ever lands, scope
"unclaimed" to the owner.

## Scope + sequence

- SIBLING-feature on #1712 — #1712's id-model + the unclaimed-set query ENABLE it; this slice
  = the picker UX (designer) + the claim-link/cursor-inherit wiring (dev). Sequenced AFTER
  #1712 (the foundation); PM files it once this UX is firm.
- Doubles as the deletion accept-vs-harden answer: ACCEPT the deletion (consequence already
  safe — non-corrupting/retained/reversible) + this dropdown is the user-facing reclaim.

## The spawn-gate is the agnostic-registration MINT-gate (cross-lane, 2026-06-12)

This dropdown's spawn-action doubles as the agnostic-registration MINT-gate (Ryan's
read-via-terminal direction + designer's mint-moment; full mechanism in PM's
`agnostic-registration-research.md`). The unification, with READ separated from MINT:

- **READ/reclaim** is on-demand off the terminal's live cwd (the OSC-7 cwd-tracker) — read
  the dir's `.bpmux-session-id` if present → reclaim. Idempotent, free, no pollution. A bare
  `cd` only moves the read-target; it NEVER mints.
- **MINT** (a fresh id, dir has none) fires ONLY at this dropdown's discrete spawn-action —
  dir-has-id → auto-reclaim (no mint); unclaimed-exist → picker (pick = reclaim, "start
  fresh" = MINT); none-unclaimed → spawn-fresh = MINT. A deliberate user-choice → zero
  reclaim-set pollution. (Dev-mode mints at MCP-connect instead — same dir→id key.)
- **READ/ROUTE client-independence** (dev-grounded 2026-06-12): the server-side cwd-read is
  reading the PTY output-stream, which today only runs when a client views the pane → a
  headless agent would be unroutable (the OSC-capture client-dependence the team already
  rejected). The cure is a SERVER-OWNED PTY-tap (bpmux always-reads each spawned terminal's
  IO, viewer-independent) — and it's the SAME substrate #1656 needs for PTY-WRITE delivery,
  so it's #1656's prerequisite architecture surfaced early, not extra cost. The MINT (at this
  dropdown, viewer-present at spawn-time even when the agent that then runs is headless) is
  unaffected; the tap is only for the running-phase read/route.

So the agnostic-trigger shape: mint@spawn-dropdown (viewer-present) + read/route@server-PTY-tap
(client-independent) + dev-mode@MCP-connect. Noted here since PM files the reclaim-dropdown +
agnostic-trigger as one vertical.

## Observation falsifier (canonical set — PM-authored 2026-06-11, designer-UX folded)

I would reject if:
1. starting in a dir that HAS an id shows a picker / mints a new instance instead of
   auto-reclaiming ("just appears"). [rests on #1712 restart-reclaim]
2. the picker shows when NOTHING's unclaimed (should be just "start fresh"), OR no picker
   when unclaimed ids EXIST. [progressive-disclosure]
3. the picker shows raw UUIDs, not display-name + last-msg-snippet + relative-timestamp
   (most-recent-first). [recognition-over-recall]
4. picking does NOT re-attach the conversation (inherit-cursor + the orphaned slice resumes
   from where it left off).
5. there's no per-terminal "reassociate" to fix a wrong-identity (auto-used-wrong OR
   picked-wrong), reusing the same picker; OR reassociate LOSES the dropped id (it must
   return to the unclaimed-set, re-pickable).
6. claiming is programmatically forgeable (must be a deliberate human pick at the view —
   security-clean; no path-derived/programmatic claim).
7. the unclaimed-set isn't LIVE-derived from #1712's RoutingIdentity (registered-id,
   no-live-terminal), or it's stale; OR "unclaimed" includes a LIVE-terminal'd id (would
   steal a live identity).

**Acceptor:** owner-walk. **Scope:** sibling on #1712; the picker + reassociate = designer
UX + dev's claim-link/cursor-inherit wiring; sequenced after #1712 (may slice claim-MVP +
reassociate-follow at filing). PM files post-#1712 off this doc + falsifier.
