# Agnostic team-message receive — design-of-record (#1656, reshaped 2026-06-11)

Designer-lane shaping deliverable for #1656 re-opened to the **true channels-independence
bar** (Ryan 2026-06-11). dev co-shaped the server-side mechanism; this doc is the
CONTRACT + ACCEPTANCE + relationship — the artifact PM scopes the slice from. The
FRAMING / SANITIZE / SECURITY half lives in `terminal-injection-format-1656.md` (referenced,
not duplicated); this doc is the RECEIVE-MECHANISM contract that consumes it.

## STATUS (2026-06-12) — reconciled to the shipped/filed substrate

Shaped 2026-06-11 with the identity-model + capture-mechanism as TO-BUILD. They've since
shipped/filed; reconciling so #1656's reshape builds on the ACTUAL substrate, not the
pre-build design (post-substrate-pivot-audit). The §contract (5 clauses + security) +
§acceptance-walk remain the #1656 receive-design (still to build, routing ON #1712 + #1713).

### ⚠️ IDENTITY-MODEL RESOLVED — 2026-06-12 (Ryan, live conversation)

A long-session loop (PM + designer pushed two over-complicated role-sources — a ＋-spawn-pick
and a marker-carries-NAME routing-key) was cut by Ryan's reframe. The settled model below
**supersedes the stale bits in the bullets that follow**:

> **⚠️ ONE FORK STILL OPEN — Ryan-pending (do not build past it; parked 2026-06-12 PM):**
> the fresh-dir MINT-TRIGGER. The documented bar is **auto-mint zero-setup** (research
> §mint-LOCUS a1, verbatim "i never set anything up"; register-only is called PRODUCT-WRONG).
> So read points 2–3 as: a dir **auto-mints a routable id + a PLACEHOLDER name the moment an
> agent runs in it** (zero-setup); `register --name` / `set_name` then REFINES the human name
> through owner-conversation — it is a NAME-refinement affordance, **NOT** the routability gate.
> "Inert-until-register" is NOT the model. The open question is only whether Ryan's later
> "register once / dir-env enough" RELAXES his own zero-setup bar (toward one-time register) or
> merely affirms "an existing marker identifies the terminal" (auto-mint stands) — **his call,
> not the team's.** Common-ground to build NOW (both models agree): drop the server-env
> `BPMUX_ROOM_AGENT` param, route-by-dir-id, read `(id, name)` from an existing marker. HOLD
> the fresh-dir mint-trigger (auto-detect-on-agent-run vs explicit-register) until Ryan settles.

1. **ROUTE BY ID, not display-name.** (Ryan verbatim: *"the routing should be by id… the id is
   the thing the router should mark on. the display name is just the display name and how you refer
   to each other. the name should be dynamic and editable."*) The dir-marker's **id is the
   route-key**; delivery marks on the id. This makes the §Identity-model target (`should_deliver`
   resolves to the id) THE model from the start — **not** an intermediate "display-routed B."
   → **Slice implication for PM (flag, not a reshape — PM's lane):** the B(display-routed)/A(id-keyed)
   split likely COLLAPSES; id-routing is the model, not a deferred A-phase.

2. **The name is a mutable LABEL — not a routing input, not a spawn-time source.** It starts as a
   PLACEHOLDER when a shell-terminal registers its dir-id, and is **forged through the
   owner-conversation**: Ryan names a fresh agent in conversation, and **the agent records its own
   name** (its own `set_name` / `bpmux register --name`), mutable thereafter. (Ryan verbatim: *"on a
   brand new one, it comes from me. then the agent needs to set it somehow. their identity is forged
   through a conversation with the owner."*) Owner-conversation + agent-owned — NOT a folder-default
   (designer's superseded (ii)), NOT a ＋-pick (PM's superseded (a)).

3. **`role=BPMUX_ROOM_AGENT` is the WRONG register source** (supersedes the line in the #1713 bullet
   below). dev grounded firsthand: `BPMUX_ROOM_AGENT` is **server-env** (server-global, can't be
   per-terminal) AND it's the **direct-spawn-command** (no shell → the agent IS the process →
   `has_foreground_job=false` → the detector never fires). So #1713 registers the **dir-id + a
   placeholder label**; the role-source is the dir + the owner-conversation, never the server-env.
   ＋ stays a **bare shell** always (Ryan: *"the plus just creates the terminal"*); the
   BPMUX_ROOM_AGENT-as-command path is the OLD configured flow — don't route the agnostic flow
   through it (no need to retire it in #1713).

**#1713 net (dev-confirmed; detector banked + 9 tests green incl. 3 integration falsifiers):**
register the dir-id (route-key) + a placeholder label for a shell-terminal; route-by-id delivery is
the #1656/#1714 keystone next. The conversation-forged name needs an agent-set affordance (`set_name`
for a Claude agent; `bpmux register --name` for a truly-agnostic one).

- **#1712 SHIPPED** (passed desk-check 2026-06-12) — the identity-model PRECURSOR: route by
  session-id at the **cursor/drain** (durable id-keyed cursor, `session/routing_identity.rs` +
  the max-clamp a840cf85), the dir-marker (`.bpmux-session-id` via `read_or_create_dir_session_id`),
  the `bpmux register` CLI, rename-via-display-update (`set_name` → `routing.rename`).
  **NUANCE (verified firsthand at desk-check):** #1712's message **DELIVERY** (`should_deliver`,
  `transport/ws/mod.rs:279`) is still **display-name string-match**, NOT id-resolve — the
  session-id is the CURSOR key, not yet the delivery key. So §Identity-model's "`should_deliver`
  resolves to:[name]→id via the durable store" describes the **#1656 TARGET, not #1712's shipped
  shape**. Req-2's full **id-keyed DELIVERY** (collision-safe — the connection carries its OWN
  id rather than resolve-from-display; a display-collision picks arbitrarily today) is **SLICE-A = #1714** (req-2's collision-safe completion — connection-carries-own-id;
  build-gated on leave-mcp-alone, sequenced after #1658). **THIS doc (#1656) is now SLICE-B,
  DISPLAY-routed only** — the agnostic-receive for today's unique-display team; #1714(A)
  completes req-2's id-keyed delivery. (The §contract's req-2 below is the FULL contract-target;
  B realizes the display-routed half, A=#1714 the id-keyed-collision-safe half.)
- **#1713 FILED** (agnostic-registration, Epic #1657, sequenced pre-#1656) — the
  §Session↔terminal-LINK capture-mechanism **EVOLVED to AGENT-START-DETECTION (B, pinned 2026-06-12)**.
  The server-owned PTY-tap monitors per-terminal: (1) cwd via OSC-7 (accumulation-buffered for
  read-boundary splits), (2) `has_foreground_job` (pty.rs:68). **TRIGGER:** a foreground-CHILD job
  sustained >~1s while cwd-stable → `read_or_create_dir_session_id(cwd)` + register(session_id,
  **placeholder-label** — NOT `role=BPMUX_ROOM_AGENT`, per the RESOLVED block above §3: the
  server-env role is the wrong source; the name is a mutable label forged through owner-conversation)
  + link_terminal; a dir that already has a marker → RECLAIM (link, durable cursor preserved). The agent does NOTHING — RUNNING in a dir IS the signal (agnostic:
  claude/codex/any script, NO harness hook — that's the #1657 point).
  **Litter-free by construction:** bare cd = shell builtin = no foreground child → NEVER mints; a
  sub-second transient (ls/cat/git) is under the ~1s gate.
  **Residual (accepted, inherent agnostic cost):** a SUSTAINED non-agent (vim/top/less) DOES mint a
  harmless INERT id (no server-side way to tell agent-from-vim without a harness signal) — bounded
  to dirs you actively ran a long program in (cd-only dirs stay clean); the reclaim-dropdown
  de-prioritizes no-history inert ids.
  The MINT is at the agent-START in the cwd'd dir, NOT the ＋-spawn — the earlier ＋-dir-pick is
  SUPERSEDED (Ryan keeps his cd-then-start flow unchanged; no spawn-UX change; he re-enters at the
  acceptance-walk).
  **Pre-registration window (cursor@job-start):** for a FRESH mint the durable cursor's initial
  position = the JOB-START timestamp (when has_foreground_job first went true, ~1s before register),
  NOT the register-instant → a message to a just-started agent in its gate-window DRAINS (reject if
  lost). RECLAIM preserves the existing cursor.
  That tap is ALSO #1656's **PTY-WRITE-delivery substrate** — ONE server-owns-terminal-IO
  architecture (read = registration, write = delivery). `bpmux register` (the CLI, #1712) remains
  the explicit/dev-mode path. So §Session↔terminal-LINK below is SUPERSEDED by this detector.

## The problem Ryan spotted (why #1656 re-opened)

The delivered #1656 receive-loop is a **Claude Code Stop hook** that drains `bpmux
wait-for-event` at the turn-boundary. It met "received-and-processed" — but it is
**Claude-specific**. Ryan (verbatim): *"if we are relying a claude hook, we have just
moved the dependency from channels to a hook"* — channels-SUBSTITUTION, not
channels-INDEPENDENCE. The independence goal (Epic #1657) fails if receive needs ANY
agent-specific mechanism.

**The true bar (Ryan):** *"there can be no agent dependent mechanisms for the real time
messaging to work. the messages need to flow into your terminal session. they need to
be addressed and received correctly."* The only agent-agnostic surface is the **terminal
(PTY)** bpmux already owns — so receive must be a bpmux SERVER concern (write into the
agent's PTY), not a per-agent harness hook.

## The contract (the falsifier-set — all five + the security-condition)

A team-message reaches an agent IFF all hold. Each is a reject-clause.

1. **AGENT-AGNOSTIC** — a NON-Claude agent (a bare shell/script in a bpmux terminal)
   receives with ZERO Claude-specific glue. *Reject:* works only for Claude Code.
   *Decisive test:* a bare-script REPL receives, no hook involved.

2. **ROUTED by STABLE INSTANCE-ID** (Ryan's "the problem" + "routing needs to be dynamic" +
   "roles are created whenever" + "you can rename yourself whenever — the role is really a
   display name" + "we are process agnostic") — the routing-KEY is a **stable per-instance
   session-id** (NOT the role-string, which is rename-fragile + collision-prone). `broadcast`
   → every live instance's terminal; `to:[name]` → resolve the display-name to its
   session-id(s), deliver only there; **no-leak**. Rename changes the DISPLAY, never the
   route; a runtime-created instance is addressable the moment it registers. *Reject:* a
   rename breaks routing; a display-name collision mis-routes; a scoped message reaches the
   wrong instance; a broadcast misses a live one; a created-whenever instance can't be
   routed to. See the **Identity-model** section below — this depends on the prerequisite
   identity-slice (PM-scoped); #1656's delivery (PTY-write) is unchanged, only the KEY moves
   from role-string to session-id.

3. **DURABLE** (reach = live OR on-reconnect, never lost) — a message sent while an
   agent's terminal is OFFLINE/mid-restart (no roster entry) reaches it when it reconnects.
   Live-push alone DROPS it — the restart-wipe class the team hit 2026-06-11 (Ryan's
   restart wiped subs; the message-loss). *Cure:* push-when-online + cursor-drain-on-
   reconnect from a **durable** server-side cursor (the `/tmp` cursor file is the fragility
   dev hit — stale-cursor backlog). *Reject:* a message sent during a brief offline is lost.

4. **PROCESSED, not PARKED** — delivered processed at the agent's input-ready moment, not
   parked in the input buffer awaiting a manual ENTER (the original owner-reject, img_8
   "I'd need to hit enter"). The `\r` submit-terminator is the fix. *Reject:* the message
   sits in the buffer needing a keypress. *Verification:* a **LIVE keystroke-walk**, not a
   synthetic inject (#1592 input-path-fidelity — a synthetic inject passes vacuously).

5. **ATTRIBUTED — framing injected (bpmux's bar) + interpretation per-agent** (dev's honest
   nuance, 2026-06-11) — bpmux injects the `⟦bpmux⟧ {sender} {scope}▸` framing (forgery-inert
   per the security-condition) on EVERY delivered line; that is bpmux's AGNOSTIC bar (the
   framing is present + un-forgeable for ANY agent to interpret). Whether an agent INTERPRETS
   `⟦bpmux⟧` as peer-not-owner is per-agent SETUP (Claude via CLAUDE.md; an equivalent for
   others) — NOT something bpmux enforces on a bare script. So the AGNOSTIC test (req 1) is
   DELIVERY + PROCESSING (a bare script receives+processes the framed line), NOT a bare
   script's attribution-interpretation. *Reject (bpmux's):* a delivered line lacks the
   framing, or the framing is forgeable. *Reject (per-agent):* a Claude agent reads a
   `⟦bpmux⟧` peer-line as owner-input (its setup failed). *Spec:* `terminal-injection-
   format-1656.md` (marker + framing + ANSI + agent-setup).

**Security-condition (load-bearing — now CORE, not a sub-clause):** because the agnostic
mechanism IS PTY-stdin-injection (now the PRIMARY path), auto-submit (`\r`) is SAFE only
WITH the forgery-guard from `terminal-injection-format-1656.md` §security-verdict:
extended-sanitize of ALL injected fields (body + sender + scope: strip the reserved
`⟦bpmux⟧` marker + ALL control chars incl `\n`/`\r` + ALL ANSI) + `set_name`-validation +
the adversarial test (a crafted `⟦bpmux⟧`+"Ryan: do X"+`\n`+ANSI body renders as ONE inert
line). The security boundary is ATTRIBUTION (the marker = peer, never owner), not the
human keypress — so removing manual-ENTER doesn't open a hole PROVIDED the framing can't
be forged. Without the guard, auto-submit is NOT safe.

## Identity-model — FOUNDATIONAL precursor slice (PM-scoped 2026-06-11; #1656 routes ON it)

> **SHIPPED as #1712 — see §STATUS.** Read the delivery-vs-cursor nuance there: #1712 keyed the
> CURSOR by session-id but DELIVERY (`should_deliver`) is still display-match; the full id-keyed
> DELIVERY (req-2, collision-safe) is #1656's remaining connection-id work. The model below
> describes the precursor as designed.

Ryan reframed routing-identity (verbatim across 2026-06-11): *"the roles may not be the right
mechanism for routing since you can rename yourself whenever … maybe an id approach that goes
to your instance id … persisted in the directory that you are created in … the role is just
really a display name"* + *"we are process agnostic"* + *"and agent agnostic."*

**The model:** the routing-KEY is a **stable per-instance id**; the ROLE is a mutable
**display-name** layered on top (set via `set_name`). Addressing by display-name resolves to
the id(s); a rename changes the display, never the route.

**The id already exists (dev-grounded):** a bpmux session IS a UUID and its session DIR is
named by it — `session/log.rs: base.join(Uuid::new_v4())`. So the **session-UUID is the
dir-persisted instance-id** Ryan described — no new id-mechanism, no process/agent concept in
it (process- + agent-agnostic by construction; the UUID is assigned to ANY process spawned in
a bpmux terminal).

**Why it's foundational (the unification — PM-credited):** id-in-dir = a stable identity
across BOTH rename AND restart. So dynamic-routing (req 2) AND durable-delivery (req 3) fall
out of ONE substrate: a reconnecting instance (same dir) reclaims its id → reclaims its
cursor → drains what it missed → no-loss. Less total work, not more.

**The seam (dev-grounded 2026-06-11) — TWO components, single-responsibility split:**

The naive "re-key `LivePtyRegistry` by session-id" is INSUFFICIENT: `LivePtyRegistry` holds
LIVE PTYs only, so an OFFLINE instance (mid-restart) has no entry — but durable-delivery +
offline-addressing REQUIRE the identity (id↔display) + cursor to be addressable WHILE offline.
So routing-identity can't live in the live-PTY roster. Split into:

1. **Durable identity-store (NEW) — the routing-identity SOURCE-OF-TRUTH** — `session-id ↔
   display + cursor`, dir-persisted (online AND offline). This is the durable-cursor home
   (closes req-3's met-falsifier gap STRUCTURALLY) and IS Ryan's dir-persisted id
   (`session/log.rs base.join(Uuid::new_v4())` = the dir name, stable across resume). It
   answers display→id resolution + holds the per-instance cursor for offline-reclaim.
2. **`LivePtyRegistry` — the LIVE-PTY delivery-map** — keyed by session-id, online-only;
   the `write_to(id, bytes)` target. Stays single-responsibility (holding live PTYs).

**Wiring:**
- **registration call-site (decisive wiring-point):** `transport/rest/room.rs`
  `pty_registry.insert(terminal_id, live)` (where `terminal_id == role` today) → at spawn,
  key `LivePtyRegistry` by the session-id + register id↔display in the durable store;
- **addressing:** `should_deliver` resolves `to:[name]`→id via the **durable store** →
  deliver via `LivePtyRegistry` IF live, ELSE the cursor holds + the event-store replays on
  reclaim; broadcast → all ids;
- **`set_name`:** updates the DISPLAY attribute in the durable store; the route-key (id) is
  preserved (rename changes display, never the route);
- **the role-throughout spots become id-dynamic:** `KNOWN_AGENTS=[dev,pm,designer,owner]`
  (rpc agent-header), `RosterRole::Owner` (nudge special-case), `Room{role}` / `RoomStore`-by-role.

**Session↔terminal LINK (capture mechanism) — DIR-DERIVATION ONLY, via an ALWAYS-PRESENT
channel (resolved 2026-06-11, Ryan + dev + designer):**

> **SUPERSEDED by #1713's AGENT-START-DETECTION — see §STATUS.** The "PTY/OSC-capture DROPPED"
> conclusion below was REVERSED: #1713's server-owned PTY-tap detects a SUSTAINED foreground-job
> in the cwd (the agent RUNNING, >~1s) → mints/registers there. NOT at the ＋-spawn, NOT on a bare
> cd (a shell builtin → no foreground child → never triggers). Agent does nothing; running IS the
> signal. Retained below for rationale-history (why the naive client-dependent capture was rejected
> — the server-owned tap is what makes the read client-INdependent).
How the server learns which session-id an agent carries (to route). The agent's id is its
**cwd at AGENT-START** — Ryan's model: a bpmux terminal spawns → the user NAVIGATES (cd) to a
dir → starts the agent THERE → the cwd-dir holds the session-id (reclaim-if-present-else-mint;
restart-in-same-dir → reclaim). So the capture is at agent-start (post-cd), NOT terminal-spawn
(which would capture the pre-cd default). The sessions-base = that cwd, NOT a fixed `~/.bpmux`.

The agent registers its cwd-derived id over an **ALWAYS-PRESENT channel** — agnostic +
NOT-client-dependent (no PTY-reader needed). Two deployment modes, ONE uniform session-id key:
- **prod** (bpmux-terminal-spawned): a **`bpmux register`-from-cwd** call (the off-MCP CLI
  surface) — works for a bare non-Claude script.
- **dev** (externally-launched + MCP, today's team): the **MCP bridge** (its CWD = the agent's
  workspace) resolves dir→id.

Both are direct calls on always-on channels → the client-dependent **PTY/OSC-capture is
DROPPED entirely** (not built speculatively — do-the-simple-thing-right). The earlier
headless-residual is MOOT: every mode (prod-terminal / dev-MCP) has an always-present channel;
neither needs a viewer. Canon-aligned: MCP+Channels for dev, terminal+PTY-delivery for prod
(#1656); #1712's session-id routing-key is uniform across both.

(Seam: `post_terminal_spawn` HOME-hardcode → the agent's base must be its start-CWD, not a
fixed path — dev grounds the base=cwd-at-agent-start derivation.)

**Bar on this slice:** process-agnostic + agent-agnostic + rename-proof + create-whenever-proof.
**Sequence:** identity-model (this) → #1656 (routes + durably-receives ON the id) → #1658 cutover.

## The approach (dev-grounded — mechanism is dev's; here for the scope's completeness)

Server-side **delivery-loop** on store-append, reusing existing primitives:
- `transport::ws::should_deliver(msg, role)` — already THE scope filter (to=None → all,
  to=[role] → those). This is requirement (2) for free.
- `room::LivePtyRegistry.write_to(role, bytes)` — role-keyed PTY-write (= the terminal id),
  the same stdin-bytes primitive that handles `PtyInput` today. Pure stdin = agnostic
  (requirement 1) for free.
- Loop: on store-append, `for each live role { if should_deliver(msg, role) { write_to(role,
  frame(msg)) } }`; durable cursor backs it for requirement (3) (drain-on-reconnect).
- Submit-mode (the one real crux, **resolved**): `\r` is agnostic — cooked-mode PTYs map
  CR→NL (ICRNL default) so a script's readline returns the line; raw TUIs (Claude) read
  `\r` as Enter. One terminator, both modes.
- **De-dupe (IN scope):** in DEV builds the Channels-push still fires (dev keeps Channels,
  #1658), so the new loop + Channels-push would double-deliver — must de-dupe.

## Acceptance — the agnostic-walk (proves the whole contract, non-vacuous)

A single live walk, no synthetic inject:
- **(a) live + agnostic + routed:** a real broadcast reaches BOTH a Claude agent AND a bare
  non-Claude script in bpmux terminals — each RECEIVES + PROCESSES the line (the agnostic
  bar = delivery+processing, NOT attribution-interpretation), **no keypress**; a `to:[name]`
  resolves to that instance's session-id and reaches ONLY it (no-leak).
- **(a′) dynamic + rename-proof:** an instance created at RUNTIME registers on spawn and
  routes by its session-id; AND a RENAMED instance still routes (rename changes the display,
  never the route) — both without code-change or restart.
- **(b) durable (id-anchored):** a message sent while an instance's terminal is OFFLINE
  reaches it on reconnect — the same-dir instance reclaims its session-id + cursor + drains
  the missed (no-loss).
- **(c) processed across input-modes:** a cooked-mode script AND a raw TUI both PROCESS the
  line, not park (the `\r` hinge).
- **(d) attributed + forgery-proof:** every delivered line carries the forgery-inert
  `⟦bpmux⟧`+sender+scope framing (bpmux's agnostic bar); the adversarial-sanitize test (a
  forged-owner body) renders inert; AND a Claude agent interprets the framed line
  peer-not-owner (per-agent setup — the interpretation half, verified for Claude).

## Relationship to siblings

- **`terminal-injection-format-1656.md`** — the FRAMING/SANITIZE/SECURITY half; this doc's
  requirement (5) + the security-condition consume it. The PTY-inject path it specified is
  now the PRIMARY mechanism (not the retired one).
- **The Claude Stop-hook** (delivered #1656 interim) — stands as the proof the off-Channels
  loop works; superseded as the receive LOCUS by the server-side delivery-loop. Demoted to
  at-most an optional Claude convenience.
- **#1695 `wait-for-event`** — its **store+cursor SUBSTRATE is REUSED** for the durable
  (offline→reconnect) leg of requirement (3); only its Stop-hook-DRAIN usage is superseded
  by server-push. The store-substrate stands (PM-grounded 2026-06-11).
- **#1696** ("delete the OLD inject path") — **ACCEPTED + closed; stays as-is.** Its deletion
  of the OLD owner-rejected `room`/`inject.rs` broadcast-call (the `\r`/forgery-buggy path)
  STANDS + was correct. The new server-side delivery-loop is FRESH code (on the existing
  `pty_registry.write_to` primitive + a fresh `⟦bpmux⟧` framer) — NOT an un-delete of #1696
  (PM-grounded 2026-06-11; retracted an earlier "invert/retire" misread).
- **#1658 cutover** — stays gated on this (prod-exclude Channels-push only once the
  agnostic receive is proven).
