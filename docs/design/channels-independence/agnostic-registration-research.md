# Agent-agnostic registration — research note (PM, 2026-06-12)

Owner-raised (Ryan 2026-06-12 00:09): *"do we have a way to register that is agent
agnostic? or do other agents have a start mechanism that can handle this? this is research
someone other than dev can figure out. dev u keep doing what you are doing."* PM-owned
research (designer deferred + stays on #1712-verify + reclaim-dropdown; available for the
trigger-MECHANISM-design half). dev stays heads-down on #1712.

## The sharpened question: the COMMAND is agnostic; the TRIGGER is the gap

`bpmux register` (from cwd, off the PART-B CLI) is ALREADY agent-agnostic — a plain CLI
command ANY shell / agent / bare script can run. The design-of-record resolved this
(`agnostic-receive-1656.md` §Identity-model: registration over an always-present channel;
the client-dependent PTY/OSC-capture was DROPPED). So "do we have an agnostic way to
register?" → **the command, yes.** Ryan's real gap is the **start-TRIGGER**: *what fires
`bpmux register` at agent-start, for ANY agent.*

**Today's trigger is Claude-specific.** dev wired the command into the **Claude Code
SessionStart hook** (`claude-plugin/hooks/session-start.sh:39-41` → `"$BPMUX_BIN" register`,
registered in `hooks.json:122-131`). That cures the team's bleed today (all-Claude) but
relies on Claude Code's hook framework — exactly Ryan's *"we just moved the dependency from
channels to a hook"* critique, one layer down. Not agnostic.

## The agnostic-trigger seam ALREADY EXISTS in the codebase (grounded firsthand)

bpmux spawns the user's **login shell** (`$SHELL -l`) and OWNS it via the retained PTY-master
(`room/spawn/pty.rs:90-139` `resolve_terminal_spawn` / `spawn_pty_retained`). And bpmux
ALREADY seeds that shell's startup:
- **zsh:** hijacks `ZDOTDIR` → a temp-dir whose wrapper `.zshrc` sources the user's real rc
  THEN injects bpmux hooks (the OSC-7 `chpwd` cwd-tracker) — `room/spawn/osc7.rs:42-73`.
- **bash:** `--rcfile <tempdir>/bpmux-bashrc` — `osc7.rs:20-40`.
- **provisioning** already appends a `claude()` agent-wrapper (MCP-config injection) to that
  seeded rc — `room/spawn/provisioning.rs:9-22`.

**So the agnostic-trigger is NOT a new mechanism — it's one line in an rc-seed bpmux already
controls.** Add `bpmux register 2>/dev/null || true` to the seeded `.zshrc`/`.bashrc` (the
same seam as the OSC-7 hook + the `claude()` wrapper) → ANY agent spawned in a bpmux terminal
auto-registers, harness-independent. **The SHELL triggers it, not the agent's hook-framework.**

## The complete agnostic picture — 3 channels, 1 uniform session-id key

| Deployment | Trigger | Status |
|---|---|---|
| **dev** (outside bpmux + MCP) | the **MCP bridge** resolves dir→id (its cwd = the agent workspace) | works today |
| **prod / in-bpmux-terminal** | the **bpmux-seeded shell rc** runs `bpmux register` (the seam above) | **the agnostic-trigger to BUILD** |
| **bare** (any agent, anywhere) | `bpmux register` manual / documented | command exists (universal fallback) |

The Claude SessionStart hook stays as a Claude-DEV convenience, NOT the agnostic path.

## Part (b) — "do other agents have a start mechanism?"

Some harnesses have a start-config (per-harness hooks/rules), but relying on each = the
**per-agent-hook path** = Ryan's own "moved the dependency to a hook" critique, N times over.
The **shell-rc trigger SIDESTEPS needing ANY agent's start-mechanism** — bpmux owns the
shell-spawn, so the shell registers regardless of which agent runs in it. That is why the
shell-seam is THE agnostic answer, not a per-harness survey.

## Why it matters NOW — dependency-order (PM-scoping)

Registration-agnosticism is the **PREREQUISITE for #1656's receive-agnosticism.** #1656's
falsifier req-1 = *"a bare non-Claude script receives."* A bare script only receives if it is
REGISTERED (has a routing-id). If registration stays Claude-hook-only, the bare script never
registers → **#1656's agnostic-falsifier FAILS at the registration step.** So the
agnostic-trigger (the shell-rc seam) should land in/with **#1712 BEFORE #1656 builds on it** —
likely a #1712 slice (the agnostic-trigger) sibling to the Claude-hook (the dev-convenience).

## The trigger-MECHANISM — RESOLVED (designer-lane, 2026-06-12)

designer resolved the trigger-timing (their flagged lane), with a cross-lane catch:
- **NOT rc-start** — fires pre-cd → captures the wrong (spawn-default) dir.
- **NOT chpwd** — mints-per-cd (reclaim-if-present-else-mint on EVERY cd) → every transient
  navigation-dir gets a spurious id → POLLUTES the unclaimed-set → floods the **reclaim-dropdown**
  with transient cd'd-dirs, not real agent-dirs. (Cross-lane: chpwd would BREAK designer's
  reclaim-dropdown — the catch that flips the recommendation off chpwd.)
- **YES — register at AGENT-START via the agent-WRAPPER.** Precedent already in the seam:
  `provisioning.rs`'s `claude()` wrapper runs at agent-start (when the user types `claude`). Add
  `bpmux register` THERE → registers only when an agent actually STARTS, in its cwd, zero
  per-cd pollution. Idempotent reclaim-if-present holds for the re-run/restart case (same dir
  → same id).
- **Full agnosticism (any agent, not just claude):** a generic `bpmux run <agent-cmd>` wrapper
  (register-then-exec), OR the seeded-rc wraps the common agent-cmds.

This KEEPS the fresh-install answer intact — the wrapper lives in the bpmux-seeded rc → ships
with the binary → zero user-setup (Ryan's "i never set anything up"). And it keeps the
reclaim-dropdown's unclaimed-set clean (only real agent-dirs). dev grounds the impl (the
wrapper wiring in `provisioning.rs`).

### Ryan refinement 2026-06-12 — server-side terminal-READ (supersedes the shell-wrapper)

Ryan (verbatim): *"what if instead a start hook, if the session is started in the bpmux
terminal, the id is in the local bpmux to that directory and read via the terminal. if it is
development, it is read into the mcp."*

The cleaner direction: a **READ (bpmux pulls), not a PUSH (the agent registers).** The agent
does NOTHING — no hook, no shell-command. It rides two seams that ALREADY exist:
- bpmux tracks each terminal's LIVE cwd via **OSC-7** (`osc7.rs` — already built);
- the directory-id persists in the **`.bpmux-session-id` marker** (dev built, #1712).

So bpmux READS the directory's id from the terminal's tracked cwd, server-side — **prod**:
via the terminal bpmux owns; **dev**: via the MCP bridge (its cwd). Most-agnostic point on the
spectrum (zero agent-involvement). This SUPERSEDES the shell-`bpmux register` wrapper above
(the wrapper was a push; this is a pull) — both ride the seeded-shell for the cwd-signal, but
the read needs no register-command.

### The mint-moment — RESOLVED (designer-lane, 2026-06-12)

designer resolved it by separating READ from MINT:
- **READ / reclaim:** on-demand off the OSC-7 live-cwd; idempotent, NO pollution (reading an
  existing dir-id is free). A bare `cd` only moves the read-TARGET — it never mints.
- **MINT** (dir has no id): ONLY at a discrete agent-START, never a cwd-change:
  - **prod** = the **reclaim-dropdown spawn IS the gate** (already designed): dir-has-id →
    auto-reclaim (no mint); unclaimed-exist → picker (pick = reclaim; "start fresh" = MINT);
    none-unclaimed → spawn-fresh = MINT. A deliberate USER choice → zero pollution.
  - **dev** = MCP-connect is the discrete start: read cwd-id → reclaim-else-mint-once.

So the mint-trigger **rides events that ALREADY exist** (the reclaim-dropdown spawn + MCP-
connect) — no new mechanism — and it UNIFIES with the reclaim-dropdown (same spawn-gate). The
pollution risk is gone (mint only on a deliberate spawn-choice, never a transient cd).

### The complete agnostic-trigger design (READ-direction + MINT-moment)

- **Command:** `bpmux register` / the routing-identity read — agnostic, in the binary.
- **READ (Ryan-direction):** server-side, bpmux pulls the dir-id from the terminal's OSC-7 cwd
  (prod) or the MCP cwd (dev) — agent does nothing.
- **MINT (designer-resolution):** at the reclaim-dropdown spawn-gate (prod) / MCP-connect
  (dev) — discrete agent-start only, zero pollution.

### PTY-tap refinement — the headless-route cure (dev-flag + designer-reconcile + Ryan-👍, 2026-06-12)

**dev's flag:** a naive "server READS the OSC-7 cwd" has the client-dependence trap we already
rejected. OSC-7 lives in the PTY-OUTPUT, and today the ONLY PTY-reader (`ws/client.rs`
PtySubscribe) runs only when a CLIENT subscribes (a viewer). A headless agent (no viewer) →
its OSC-7 is never read → unroutable. Viable ONLY with a **server-owned PTY-tap** — bpmux
always-reads each spawned terminal's output, client-independent.

**designer's reconcile — MINT and READ are orthogonal, so there is NO headless-MINT gap:**
- **MINT/registration** fires at SPAWN via the Electron reclaim-dropdown gate — a VIEWER IS
  PRESENT at spawn-time even if the agent that then runs is headless, so the dir-id is minted
  BEFORE the bare-script runs. NOT OSC-7-driven → no headless-mint hole.
- **READ/ROUTE** (running-phase, headless, no viewer) → the **server-owned PTY-tap**.

**Ratified mechanism (dev, Ryan-👍):** on `cd`, the shell emits its new cwd (the OSC-7 the
terminal-integration ships) → bpmux's server-PTY-tap sees the cwd-change → reads the
`.bpmux-session-id` marker in that dir → auto-registers (reclaim). The agent does NOTHING (just
`cd`s); bpmux reads the dir server-side. Fresh dir, no marker → mint at the spawn-gate (viewer
present). The marker built in #1712 is exactly what gets scanned.

**Settled shape:** MINT @ spawn-dropdown-gate (viewer-present) + READ/ROUTE @ server-PTY-tap
(client-independent: OSC-7 cwd-change → `.bpmux` marker → auto-register) + dev-mode @ MCP-
connect (no tap). The **server-owned PTY-tap is the ONE net-new piece** the agnostic-trigger
ESTABLISHES; #1656 reuses+extends it for the PTY-WRITE delivery — same server-owns-terminal-IO
architecture (read+write, client-independent), so it's shared, NOT extra cost.

PM hand-off: RESEARCH + DIRECTION + MINT + PTY-tap all settled + Ryan-👍 (placement: pre-#1656
slice, post-#1712). Remaining: file the **agnostic-trigger slice** post-#1712 — establishes the
server-owned PTY-tap + OSC-7-read→`.bpmux`-marker→auto-register + mint-at-spawn-gate; dev
grounds+builds. Unifies with the reclaim-dropdown (shared spawn-gate) — sequence/slice the two
together at filing (the tap may be sized as the slice's vertical-foundation, reused by #1656).

### The DISPLAY-source — RESOLVED (PM model-call, 2026-06-12, #1713 commit-2 fork)

dev's commit-2 fork: `register(session_id, display)` needs a `display`, but the marker holds only
the session_id (UUID). Where does `display` come from? **Resolution: `display` ≡ the terminal's
room_agent role — ONE addressable-identity axis, sourced at MINT, durable in RoutingStore; the
tap re-LINKs.**

- **Why role IS the display:** `should_deliver` is a DISPLAY-match — a `to:["dev"]` routes to
  whatever registered with display `"dev"`. That addressable name is *exactly* what the
  room_agent role already is. The mint-moment (spawn-gate) has the role in hand (`BPMUX_ROOM_AGENT`,
  per dev's ground), so mint sets `display = room_agent`. The tap-on-cd then re-LINKs
  (read marker → link session_id→terminal; display already durable) — the READ supplies NO display.
- **Why NOT a marker-carried-display / per-dir config (dev's option b):** that mints a PARALLEL
  identity-source alongside the role — two-sources-of-truth (the masked-horizontal smell) — for a
  case prod doesn't produce. If a terminal is addressable at all, it has a role; if it has none,
  it is not a route-target by ANY mechanism, so a config-display buys nothing.
- **It STRENGTHENS #1656 falsifier-1** (the bare non-Claude script receives): the script receives
  precisely because its TERMINAL carries the role (= its display); the script stays inert. Does
  NOT shift the falsifier (designer-flag pending).
- **Lane:** the register-vs-link API choice + the provisioning wiring is dev's impl-ground; this
  fixes only the identity-model (where display comes from), not the wiring.

### ⚠️ SUPERSEDED (2026-06-12) — final mint-locus = AGENT-RUN DETECTION (not ＋-spawn-targeting)

The mint-locus resolution below (the ＋-spawn-targeting / Option-1 / a1 / spawn-gate-mint) is
SUPERSEDED. Ryan declined the ＋-change (2026-06-12: *"where it opens is fine, i just cd"*); the
team converged on **server-side AGENT-RUN DETECTION**: the server-owned PTY-tap detects a SUSTAINED
foreground-CHILD job in the cwd (>~1s, via `has_foreground_job` `pty.rs:68` + the OSC-7 cwd) →
`read_or_create` the cwd's id + register(role) + link. Bare `cd` is a shell builtin (no child) →
NEVER mints (litter-free by construction); sub-second transients (ls/cat/git) fall under the gate;
a SUSTAINED non-agent (vim/top/less) mints a harmless INERT id (the inherent agnostic cost — no
server-side agent-vs-vim distinction without a harness signal; the reclaim-dropdown de-prioritizes
no-history ids). The ＋ stays HOME-only — **Ryan's flow is UNCHANGED, zero-setup, and the spawn-UX
OWNER-GATE is REMOVED** (no ＋-change → no owner-decision; he re-enters at the acceptance-walk). The
cursor's initial-position = the agent JOB-START, so a `to:[role]` sent in the ~1s pre-registration
window still drains once registered.

**Still holds from below:** display ≡ room_agent role (the DISPLAY-source section above), the
server-owned PTY-tap (the read-substrate), reclaim-on-return (dir has a marker), no passive
cd-litter. **Superseded:** the MINT fires at agent-run-detection (NOT a ＋-spawn-gate) — so there is
no ＋-targeting and no spawn-dir-vs-work-dir seam (the agent runs where it works).

**Final spec: #1713's body** (reconciled 2026-06-12). The ＋-spawn-targeting exploration below is
retained as design-history (why ＋-targeting was considered, and how Ryan's flow-preference + the
litter-vs-foreground-job analysis pivoted it to agent-run detection).

### The MINT/LINK-flow invariant — the spawn-dir vs work-dir seam (designer-catch + PM-resolve, 2026-06-12) [SUPERSEDED — see banner above]

designer's catch: the architecture has TWO identities — **display** (role) bound to the
TERMINAL (stable across cd), and **session_id** (cursor) per-DIRECTORY (the `.bpmux-session-id`
marker). So `register(session_id, display)` binds *this dir's cursor* to *this terminal's role*.
Risk: if MINT fires at ＋-spawn but the agent then `cd`s to a different work-dir before starting,
the spawn-gate minted the SPAWN-dir's session while the agent works in the WORK-dir — and
falsifier-3 forbids the tap from minting-on-cd to paper the gap.

**Invariant (PM model-call):** MINT (the dir session_id-marker AND the display=role binding,
together) fires ONCE at the discrete spawn-gate, FOR the dir the agent works in. The tap-on-cd is
RECLAIM-READ + LINK only — NEVER a cd-mint.

**Why it holds — Ryan's flow-description settles the seam:** *"the id is local to that directory
[where the session is started] and read via the terminal."* The id mints in the WORK-dir (where
the agent starts), and the terminal reads it. So the spawn-gate IS the work-dir — there is no
spawn/work split, hence no cd-mint.

**The one dev-ground (start_story):** verify prod's spawn-flow lands agent-work-dir ==
spawn-gate-dir. IF grounding finds spawn→cd lands a FRESH unmarked work-dir (a real spawn≠work
gap), do NOT cd-mint to cover it — SURFACE it (it touches Ryan's described flow → re-model /
owner-confirm). falsifier-3's no-cd-mint rule exists precisely so this gap can't be silently
papered.

**Falsifier-wording (pin at start_story):** falsifier-1 = the bare-script is routed by its
SERVER-ASSIGNED room_agent-role (strengthens "zero agent glue"); falsifier-3 = the tap
reclaim-reads/links (session-id↔terminal), never mints on cd.

### The mint-LOCUS sub-fork — RESOLVED (a1), and the work-dir-mint durable hinge (2026-06-12)

dev grounded the crux: `read_or_create_dir_session_id` (log.rs:184) is mint-OR-reclaim — the
#1712 Claude-hook minted the WORK-dir marker POST-cd (at the agent's cwd). A reclaim-only tap
has nowhere to mint a first-time work-dir. Sub-fork:
- **(a1) spawn-gate spawns IN the work-dir** (cwd=work-dir → mint there).
- **(a2)** assumes the work-dir marker pre-exists from a prior spawn; first-mint out-of-band.
- **(a3)** tap read_or_creates (mints-on-cd) — forbidden by falsifier-3.

**Resolution: (a1) — and (a2) is (a1)'s STEADY-STATE, not a rival.** Product-anchor: first-time
zero-setup auto-register IS the core requirement (Ryan "i never set anything up"). (a2)-standalone
FAILS it — "brand-new work-dir unregistered until a prior mint" is exactly the bleed we cure; it's
falsifier-clean but product-wrong. (a3) is the pollution falsifier-3 forbids. (a1) is the only path
that is BOTH first-time-registering AND pollution-free: the mint rides a DELIBERATE server-visible
event the user already does — the spawn-gate TARGETING the work-dir. First mint = "start fresh in
<work-dir>" at the spawn/reclaim-dropdown (cwd=work-dir). Re-entry = reclaim the now-existing
marker (= (a2)'s steady-state). Tap stays reclaim-only. Zero-setup because the WHERE-does-the-agent
-work pick at spawn IS the registration — no hook, no shell-cmd.

**The durable hinge (designer):** offline→reconnect reclaim needs the marker IN the work-dir. A
fresh-work-dir keeps the spawn-session-id for LIVE routing, but DURABLE reclaim only holds if the
mint landed where the agent WORKS. → the mint MUST land in the work-dir.

**Grounding-state (dev, CONFIRMED firsthand 2026-06-12):** `post_terminal_spawn` (rest/room.rs:17)
takes NO dir param (Extensions-only) and hardcodes `cwd=HOME` (line 49) — the ＋ is HOME-only
end-to-end, no dir-targeting anywhere in the spawn path. So the spawn≠work gap is REAL: a
mint-at-＋-spawn mints HOME, the agent cd's away, the work-dir never first-mints → durable reclaim
breaks. The mint MUST land in the work-dir. Surfaced per the call (NOT cd-mint to paper it).

**Scope resolution (PM call — Option 1):** the minimal dir-targeting rides IN #1713 —
`post_terminal_spawn` gains a dir param + the ＋ supplies the agent's work-dir at spawn. Vertical-
slice integrity: the server-tap is INERT without a spawn that targets the work-dir (infra-alone
otherwise; #1713's own durable-reclaim falsifier unmeetable). The full reclaim-dropdown UI (picker
/ unclaimed-set / recognition-cues) is a SEPARATE designer-story on top — the minimal dir-pick IS
that dropdown's core spawn-gate (designer-confirmed), enriched later, not a throwaway.

**OWNER-GATE (commit-2):** the ＋ gaining work-dir-targeting is a visible spawn-UX change on Ryan's
flow → surfaced to Ryan (recommendation: minimal dir-target at ＋-spawn). designer captured the
spawn-gate dir-pick IA + a structural sketch (`reclaim-dropdown.md` §Spawn-gate dir-targeting):
＋ → PICK the work-dir (recent ▾ + browse) → the dir's marker-state drives the existing
reclaim-states (auto-reclaim / picker / fresh). Ryan confirms the DIRECTION off the sketch now; the
pixel-faithful visual is app-gated (grounded against the LIVE Sidebar render when the app's up —
no phantom-fidelity). commit-2 (mint+register+tap wiring) builds once Ryan confirms the spawn-flow
direction; commit-1 (server-owned PTY reader + fanout) is landed + behavior-preserving regardless.
