# bpmux Roadmap

**Thesis:** bpmux is collective memory for human-AI teams. The persistent log is the primitive. Everything else is built on top of it.

---

## Active iteration pipeline

The serial iteration pipeline + planned-horizon entities. Each iteration runs serially (one-after-another); the next opens when the previous closes terminal. **This section is the durable cross-gap source-of-truth for iteration-order** until the briefing-persistence fix (#1563) lands (the briefing_e2e integration test pollutes prod `~/.bpmux/briefings` without `BPMUX_BRIEFINGS_DIR` isolation, clobbering pm.md + dev.md continuity across sessions).

Last updated: 2026-06-13. **ITERATION LUPUS #1694 CLOSED (acceptance ceremony 2026-06-13)** — 11 ACCEPTED: 9 team-class CLI-verb slices (#1699/#1711/#1709/#1710/#1698/#1701/#1695/#1712/#1700, each on fresh dual dev+designer verdicts) + 2 owner-walk by Ryan (#1713 auto-register · #1593 send/read-in-stream). 1 REJECTED at owner-walk: **#1656** (agent-agnostic receive keystone) — falsifier-4 INTERMITTENT raw-TUI submit-park (a pushed message lands in the terminal input but does NOT auto-submit; root broader than the cold-start race — Ryan: intermittent throughout) → **CARRIES to the next iteration** for the deterministic-submit rework (reject-reason on the story; dev owns — deterministic submit independent of agent-state + a test reproducing the busy/cold-start race; the /bin/sh test pre-emits the toggle = an input-regime-proxy that masks it). The channels-independence **owner-surface is CLI-complete + #1700-parity-proven** (no agent-verb strands off-MCP; Ryan ran the ENTIRE acceptance from bpmux off-MCP). **SEQUENCING (PM-owned, team-grounded):** the no-MCP production cutover (#1658) is GATED on #1656's rework — without MCP the SOLE push-receive path IS #1656's PTY-injection (the rejected park) and MCP-receive was the backstop; so the **next iteration LEADS with #1656's rework → which unblocks the no-MCP cutover (#1658)**. **Retro 2026-06-13 banked META-CANON** `proxy-trap / claim-grounding under pace` (always-on claim-FORMAT [observable/green/honest split at author-time] + per-class RULES [transport-ambiguous-receipt · input-regime-proxy · board-render-proxy] + peer-ground tag-verify backstop; NO discipline-leg — mechanism>discipline). **Retro affordance/Bug candidates:** stale-board-render (Bug — render lagged substrate, misled board-reads 2×) · reject-context board-marker (a rejected-rework story looks identical to fresh-ready) · grounding-gate re-arm friction (per-session ground-mark should persist). **SEQUENCE: next-iter (#1656-rework-led) → Crux #1707 → Darcula (#1678→#1679→#1681) → Tucana → Hydra.**

**PRIOR-STATE 2026-06-12 (historical) follows:** **#1712 desk-check-PASSED 2026-06-12** (identity-model foundation: route-by-session-id + rename-proof delivery [pinned at should_deliver, the WS-e2e at the delivery-locus] + register-from-cwd e2e + durable cursor; held at desk-check-passed — team-class, batch-at-drain). → **#1713 IN-PROGRESS 2026-06-12** ([Feature] agnostic-registration Slice-1, Epic #1657): the server-PTY-tap REPLACES the client-subscribe PTY-reader (`ws/client.rs` PtySubscribe) with one server-owned reader per terminal that fans-out to subscribers (the headless-route cure, shared with #1656's PTY-WRITE) + agent-run-detection register: on a SUSTAINED foreground-CHILD job in the cwd (>~1s, via `has_foreground_job` pty.rs:68 + OSC-7) → `read_or_create` the dir's id + register(role) + link; a bare `cd` (shell builtin, no child) NEVER mints (litter-free by construction); ＋ stays HOME-only — **Ryan's flow UNCHANGED** (the earlier ＋-spawn-default-mint framing is SUPERSEDED per the story's 2026-06-12 design-pivot note; the spawn-UX OWNER-GATE is REMOVED). ONE vertical (tap ships WITH its consumer). Load-bearing falsifier = the live client PTY-view streams unbroken post-refactor (#1712 falsifier-7 made REAL). **commit-1 LANDED (80200e3e, server-owned reader+fanout, CI-green; also fixes the latent N-viewer fd-split); commit-2 BUILDING (agent-run-detection register; OSC-7 accumulation-buffer + cursor-floor banked green). Sequenced ready-pos-1 + Lupus-member.** **Sequence now: #1712 → #1713 → #1656 → #1658** (#1713 BEFORE #1656 — the tap IS #1656's substrate + the bare-agent-registration prereq). Agnostic-trigger Slice-2 (reclaim-dropdown picker = mint/reclaim CHOICE) stays drafted (reclaim-dropdown.md, far-ahead). CI-runtime speedup (cargo-audit-prebuilt + cargo→nextest + a 2nd self-hosted-runner instance) = dev-chore after #1712 (Ryan-valued fast-feedback, quality-coupled). Design-of-record: `agnostic-registration-research.md`. — PRIOR (RECEIVE RE-OPENED 2026-06-11): **#1656 RE-OPENED 2026-06-11 (owner-raised independence gap):** Ryan walked the off-Channels receive and found the Claude-Stop-hook mechanism is channels-SUBSTITUTION, not channels-INDEPENDENCE — a Claude Code Stop hook is Claude-SPECIFIC, so relying on it just MOVES the dependency (Channels → a Claude hook). Ryan: "we are closer but not independant ... there can be no agent dependant mechanisms ... the messages need to flow into your terminal session ... addressed and recieved correctly." The receive reworks onto the agent-agnostic surface bpmux ALREADY owns — the terminal PTY: a bpmux SERVER-SIDE delivery-loop (on store-append, for each live role `if should_deliver(msg,role) { write_to(role, frame(msg)) }`), AGNOSTIC by construction (stdin-bytes any agent reads), ROUTED by STABLE SESSION-ID (the #1712 identity-model precursor: route-by-session-id not role-string — rename/restart/create-proof, role = display-name), DURABLE (push-online + cursor-drain-on-reconnect — no-loss across offline/restart, the restart-wipe class), PROCESSED (`\r` submit — cooked CR→NL + raw Enter, one agnostic terminator), ATTRIBUTED (re-carry ⟦bpmux⟧ framing — PTY-input isn't inherently sender-attributed). Falsifier = an agnostic LIVE-walk (a non-Claude bare-script AND a Claude TUI both receive a routed broadcast no-keypress + to:[role] reaches only that role + a message sent while a terminal is OFFLINE reaches it on reconnect). **#1712 (identity-model precursor) FILED ready-top** (route-by-session-id, role-as-display, awaiting-team) → #1656 routes on it (reshaped to route-by-id) → #1658. dev pulls #1712 red-first (rename-proof + offline→reconnect tests, then the rest/room.rs roster re-key + set_name→display + static-spot migration). Design-of-record: agnostic-receive-1656.md (+ §Identity-model). **SIBLING-FEATURE on #1712 (owner-raised 2026-06-11, shaped, NOT yet filed — files POST-#1712):** a **reclaim-an-unclaimed-conversation dropdown** — at the Sidebar ＋ spawn, if the spawn-dir has no id AND unclaimed ids exist (registered session-id, no live terminal), a one-step picker (recognition-cue rows: display-name + last-msg-snippet + relative-ts, most-recent-first) lets the owner reclaim an orphaned conversation-slice (ties to that session-id + inherits its cursor); + a per-terminal "reassociate" correction-path; immediate-pick, no confirm (reversible). Doubles as the deletion accept-vs-harden answer (ACCEPT the deletion — non-corrupting/retained/reversible — this dropdown is the reclaim). Acceptor: owner-walk. Design-of-record: `docs/design/channels-independence/reclaim-dropdown.md` (designer-UX + PM 7-clause falsifier); may slice claim-MVP + reassociate-follow at filing. The Stop-hook/`receive-broadcast.sh` bash is SUPERSEDED as primary (designer's bash class-cure moot); #1696's PTY-delete STANDS (dev builds a FRESH loop on the existing pty_registry.write_to, not un-deleting it); #1695's store+cursor is REUSED for the durable leg (only its Stop-hook-DRAIN role goes). — **Lupus #1694 PART-B (CLI re-homing) COMPLETE** — the universal `bpmux <verb>` CLI re-homing fully DELIVERED (10 slices: PART-A #1695/#1656 off-Channels receive + PART-B #1593/#1698 reads + gated-write #1699-3a/#1711/#1709/#1710 + capture #1701 + canon/iteration #1700 with its off-MCP completeness KEYSTONE-audit — 53 verbs + 13 reasoned-exclusions, empty in-scope diff over the 66-tool DISPATCHED registry, non-vacuous [deliberate-RED proven]). A prod-agent off-MCP strands on NOTHING. All delivered + desk-check-PASS, accumulating for batch-accept. REMAINING in Lupus = the **#1712 identity-model precursor** (route-by-session-id; awaiting-team) → the re-opened agent-agnostic **RECEIVE #1656** . **#1658 (production cutover) DEFERRED OUT of Lupus 2026-06-12 (Ryan @pm directive: "don't worry about the production cutover until we need to go to production … not important for this iteration; we'll go to production soon but separately").** Moved ready→icebox + unparented from #1694; re-homes to a near-term PRODUCTION-GO step (placement at a future plan-step — NOT minted preemptively). Still gated on #1656-proven + conditionally #1714(A) per its body. So **Lupus's remaining build-scope = #1713 (in-progress) + #1656 (ready)** + the delivered batch. Sequence now: #1713 → #1656. NEXT-OWNER-GATE: Ryan's **agnostic-receive walk** (live + scoped + offline→reconnect no-loss, rename-proof, non-Claude proof). Batch-acceptance of the delivered PART-B slices remains Ryan's discretion. Then Lupus closes → Crux #1707 (production-go re-homes near-term, post-Crux or as Ryan times it). **CHANNELS-CANON CLARIFIED (Ryan 2026-06-08, BINDING):** MCP + Channels STAY for DEVELOPMENT; PRODUCTION doesn't NEED them — the arc is ADDITIVE (add the prod CLI), NEVER remove-for-dev. So **#1658 = PRODUCTION-EXCLUDES the Channels-push (dev keeps it)**, not remove-for-all (retitled). The dead TUI-tools (capture_screen + simulate_*, dead since #1166's TUI-deletion) were cleaned + owner-APPROVED separately (dead-surface ≠ the dev-surface; the gate-skipping-REST class — post_close_iteration/post_accept_story — must route via the GATED handler, the #1700 keystone-audit's load-bearing check). **NEXT after Lupus = Crux #1707** (the roadmap-instrument; PM-ready — 5 members #1703/#1704/#1684/#1685/#1705 shaped+ordered+acceptor-labeled + Ryan-design-approved). Prior — 2026-06-07: (**Iteration Aquila #1655 CLOSED** — 7 accepted: 3 Grus-rejects #1517/#1518/#1524 [drag-saga] + the reject-record arc #1683/#1686/#1687/#1691; the channels-INDEPENDENCE anchor #1656/#1658 CARRIED into **Iteration Lupus #1694** (created 2026-06-07 — the channels-iteration the Aquila-close should have left; stay-ahead oversight, owned+fixed). **NEXT = Lupus #1694** [Epic #1657]). Prior 2026-06-06: Grus CLOSED; Pyxis #1521 RETIRED; Aquila was current — true channels-INDEPENDENCE: replace Claude Code's Channels transport with a bpmux-native speak-into-terminals path. Reconciled after a multi-mis-read this session: the planned "channels-independence" (Pyxis #1521) was built on the rooms-as-homes model the native-terminal pivot superseded — its core #1602-1605 was already delivered (Puppis #1614) or owner-reverted (#1604/#1605 build-ahead), and PM also wrongly minted Pictor #1654 (retired). The REAL owner-RAISED problem — Claude's experimental Channels feature still carries ALL agent-delivery — anchors Aquila #1655, now shaped into **Epic #1657**. (SUPERSEDED 2026-06-07: "architecture LOCKED on store-push-injection" + the #1656 owner-reject belonged to the PTY-injection frame. Once the team grounded that bpmux's durable-event store is already built (#1631 children #1632/#1634/#1635 accepted), #1656/#1658 reshape onto #1657's subscribe/wait_for_event slices — see the NEXT row + the #1657/#1631 comments.) The design-system is an EPIC discussed-this-iteration → planned-later (NOT an iteration; NOT a vehicle for the rejects). This roadmap is PM's to maintain so the next clear grounds in STORY-STATES here, not the briefing.

### Current pipeline (post-pivot)

**PIVOT 2026-06-02:** the rooms-as-homes plan (Epic #1320 — Apus rooms-spawn / Dorado Track-A / Pyxis Track-B/C/D) PIVOTED to **native-shell-terminals** at Carina (Ryan's room=terminal → native-terminal course-correction). The pre-pivot serial-pipeline table below is **HISTORICAL**; the rooms-pipeline is reconciled into the native-terminal arc at the next IPM.

| # | Iteration | Theme | Status |
|---|---|---|---|
| — | **Carina** [#1601](https://github.com/RyanDur/bpmux/issues/1601) | Native-shell terminals (the pivot) | **CLOSED 2026-06-02** — 7/7 features owner-accepted (spawn $SHELL · resize · N-terminals · lifecycle · close/kill · live-cwd · Cmd-C) + verdict-freshness + #1467-cleanup |
| — | **Vela** [#1613](https://github.com/RyanDur/bpmux/issues/1613) | Native-terminal native-feel completion | **CLOSED 2026-06-03** — 7/7 accepted: #1608 survive-refresh (re-attach by terminal_id) · #1607 roadmap-fix (exe-anchored repo-root) · #1609 keyboard-rename (F2 + ref-free focus-lifecycle) · #1610 Nerd-Font (leverage-system, no bundle) · #1611 bottom-clip (structural inner-wrapper) · #1606 bash-OSC-7 chore · #1603 A2A-spike → build-native decision |
| — | **Grus** [#1544](https://github.com/RyanDur/bpmux/issues/1544) | UI/render polish | **CLOSED 2026-06-05** — 6 accepted; 3 visual-rejects (#1524/#1517/#1518) carried → added to channels-independence below |
| — | ~~channels-independence = Pyxis #1521~~ | rooms-as-homes Track-B/C/D | **RETIRED 2026-06-06** — built on the rooms model the native-terminal pivot superseded; core #1602-1605 delivered (Puppis #1614) or owner-reverted (#1604/#1605 build-ahead). Members → Aquila. |
| CLOSED | **Aquila** [#1655](https://github.com/RyanDur/bpmux/issues/1655) | **Channels-independence** — Epic [#1657](https://github.com/RyanDur/bpmux/issues/1657): bpmux fully Claude-INDEPENDENT (off Channels + off MCP) via its OWN store + CLI | **CLOSED 2026-06-07** — 7 accepted (3 Grus-rejects #1517/#1518/#1524 + reject-record arc #1683/#1686/#1687/#1691); theme-anchor #1656/#1658 carried into **Iteration Lupus #1694** (created 2026-06-07). Architecture **LOCKED 2026-06-06** (constraint-derived: No-polling + no-Channels + keep stock-CLI + Redux): **STORE** (SubscriptionRegistry+dispatch = Redux SoT + role/addressee FILTER) → **DELIVERY = PUSH-via-injection** (`write_to` into the role-matched terminal) + the **misattribution-guard** (sanitize/frame, forgery structurally impossible). Design-of-record: `docs/design/channels-independence/{architecture,dev-architecture}.md` + `terminal-injection-format-1656.md`. Slices (PTY-injection frame — SUPERSEDED 2026-06-07, see NEXT row): **#1656** PARTIAL (its `broadcast_received` structured-event dispatch LANDED = slice-1 done, commit f0c4eefe; but its PTY-injection DELIVERY was owner-rejected 2026-06-06 → RETIRE-not-repair: delete the room/inject.rs path) → **#1658** cutover (remove the Channels push, GATED on the receive-loop) → **MCP→CLI re-homing = NEXT-priority iteration** (epic slice-4: every MCP tool → `bpmux <verb>` + PreToolUse→Bash-hook gate-migration). Scoped `to:[role]` + identity-map = deferred increment (#1615/#1602-fork, Ryan-reserved). RIDING: 3 Grus rejects #1524/#1517/#1518 (current tokens). Rooms-stories #1326-1334 DISPOSED: #1326 retired (subsumed by #1656) · #1330-32 deferred (agent-diversity = own future iteration) · #1327/#1328/#1333/#1334 deferred · #1329→Tucana. #1593 = the OUTGOING `bpmux <verb>` visible-surface (re-scoped into MCP→CLI). Design-system EPIC discussed-this-iter → planned-later. |
| **NEXT** | **Iteration Lupus [#1694](https://github.com/RyanDur/bpmux/issues/1694)** — channels-INDEPENDENCE (Epic [#1657](https://github.com/RyanDur/bpmux/issues/1657)) | **Claude-INDEPENDENCE** — broadcast off Claude's Channels (received via bpmux's own store) + a universal `bpmux <verb>` CLI in dev+prod (MCP retained as a dev-only supplement) | **OPEN + current 2026-06-07; dev-OK'd. PART A ready; PART B (universal CLI) shapes as PART A drains.** Members (dep-ordered, all in ready): PART A — #1695 (2a `bpmux wait-for-event` primitive) → #1656 (2b Stop-hook receive-loop) → #1696 (cleanup: delete redundant PTY path) → #1658 (slice-3: prod-EXCLUDES the Channels-push, dev keeps it — gated on the receive-loop); PART B universal CLI — #1593 (1: send+read) → #1698 (2: board reads) → #1699 (3: gated writes, owner-superset-bypass) → #1701 (4b: walk/Electron) → #1700 (4a: canon/iteration + completeness parity-audit, last). SUBSTRATE GROUNDED 2026-06-07 (PM, firsthand — the prior "store-push-injection / #1631-passive-push fold-in" framing was STALE). The durable-event store is ALREADY BUILT: #1631's children **#1632/#1634/#1635 ACCEPTED** (persist-across-restart · replay-w/cursor · provisioned live-push), only **#1643** (addressed-msg replay) remains icebox. So the channels-off core is SMALL and rides that store (dev firsthand-grounded 2026-06-07): **slice-1 DONE** — broadcast-chat already dispatched as a `broadcast_received` structured event (#1656 commit f0c4eefe, connection.rs:257-288, tests passing) → **slice-2 (KEYSTONE)** the receive-loop → **CLEANUP** delete the redundant LIVE PTY-injection path #1656 also left (room/inject.rs → ws/client.rs:64 — RETIRE-not-repair; three delivery paths run at once today) → **slice-3** remove the Claude Channels push (connection.rs:242-256, gated on the receive-loop proven reliable). **#1656/#1658 reshape onto these slices** — the PTY-injection frame + its injection-guard/format/mockup RETIRE (structured events are inherently sender-attributed; the misattribution-guard moots). **Receive-model crux RESOLVED** (dev's mechanism call): a Claude Code **Stop hook** drains `bpmux wait-for-event` at the turn-boundary — harness-fired (not agent-memory → not fragile), blocking-PULL (not poll), stock hook-contract (not own-the-loop); inside all of Ryan's standing constraints + his "you should be able to subscribe to events." **(SUPERSEDED 2026-06-11 — see the RECEIVE RE-OPENED lead: a Claude Stop-hook is Claude-SPECIFIC = channels-SUBSTITUTION not independence; the receive reworks to a bpmux SERVER-SIDE agent-agnostic PTY-delivery loop — agnostic+routed+durable+processed+attributed.)** Open sizing-flag (dev): whether the Stop-hook loop needs a `bpmux wait-for-event` CLI first (a bite of slice-4 pulled forward) or can drain the existing path now. **MERGED + refined 2026-06-07 (Ryan):** Lupus carries the FULL Claude-independence Epic — **PART A** off-Channels broadcast receive (the 4 slices above, ready, drains first) + **PART B** a universal `bpmux <verb>` CLI (agent send/act in BOTH dev+prod, built production-complete: #1593 thin-first → board-reads → gated-writes → namespaced/walk, off `cli-surface-grammar.md`; every gate + server-side role-guard preserved; dev mechanism, designer pre-pickup-reviews; joins ready as PART A drains). **MCP STAYS** as a dev-only supplement (Ryan: "leave the mcp server and client alone … cli for production, mcp for development … cli available for both") — **NO off-MCP cutover.** HELD: #1616/#1617 (build-ahead). SEPARATE: **#1697** briefing-staleness fix · **computed-style/DOM-inspect read-tool** (raise with Ryan). Full frame: Lupus #1694 body + #1657 comments. |
| N+2/3 | horizon (notional) | CI/runner-infra design-call (e2e-on-push:main bit twice — verification-integrity gap; Ryan-axis) · #1320 dead-vs-deferred audit + native-terminal Epic restructure (Ryan-gated) · #1612 scrollback-replay (promote if Ryan confirms history is felt-job) · MCP-tools-into-terminal (Dorado-carry, folds into #1602) | Notional |

### Serial pipeline (pre-pivot, HISTORICAL — Microscopium → Apus → Dorado → Hydra)

| # | Iteration | Entity | Theme | Status | Slices (~pt) |
|---|---|---|---|---|---|
| 1 | **Microscopium** | [#1398](https://github.com/RyanDur/bpmux/issues/1398) | Team-substrate foundation (isolate-from-production + rooms-spawn-start) | CURRENT (designated; awaiting Ryan's start-clearance) | #1563 briefing[1] → #1564 harness-loop[2] → #1569 harness-extend[1] → #1319 channel-history-loss[2] → #1571 spawn[2] = **~8pt** |
| 2 | **Apus** | [#1573](https://github.com/RyanDur/bpmux/issues/1573) | Rooms-spawn continuation + channel-canonicalization | Planned (opens after Microscopium closes) | #1572 list[1] · #1566 read[2] · #1567 kill[1] · #1568 canonicalization[1] = **~5pt** |
| 3 | **Dorado** | [#1574](https://github.com/RyanDur/bpmux/issues/1574) | Rooms-as-homes Track-A continuation (Epic #1320, Slices 1b → 3) | Planned (≤2pt splits at IPM) | #1466 owner-view · #1467 agent-broadcast · #1468 multi-room · #1469 permission-prompt (each 3+pt → vertical splits at IPM) |
| 4 | **Hydra** | [#1575](https://github.com/RyanDur/bpmux/issues/1575) | Agent-CI tooling (Epic #1308) | Planned | #1310-15 (6 bridge MCP-tool slices, mechanical-replication from archetype #1309) |

### Post-pipeline planned horizon — priority-ordered by OUTCOME

Per the outcome-oriented-roadmap practice (labspractices): each entry is the OUTCOME (the behaviour-change/why), ordered by value×effort+risk; mechanism/slices are flexible beneath it. Serial — each opens at its own IPM after the prior closes.

**Flow + priority (Ryan 2026-06-06):** new explorations (the in-bpmux design-review-surface / Figma-like canvas affordance, and other surfaced ideas) come via **Ryan + designer SHAPING → PM PLANS** when they bring a converged design. They are FURTHER-notional and do **NOT** prioritize over the current/planned iterations — we get to the planned arc (channels-independence → design-system) first, not push it out. PM elaborates the future into sized iterations **at the plan-stage, not preemptively** (the horizon stays ~3-notional ahead; the design-system's internal slice-decomposition lives in Epic #1659, not as separately-placed roadmap-iterations). Stale/uncreated horizon entries (Reticulum, Octans) are dropped — see notes below.

1. **N+1 — Claude-INDEPENDENCE** [Epic #1657]. *Outcome:* the team's comms run off Claude's experimental Channels AND tooling runs off MCP — zero dependency on a Claude feature that drops events silently / can be disabled out from under us. *Completion:* #1658 Channels-cutover (gated on #1656 landing) + MCP→CLI re-homing. *IMPACT (raised 2026-06-06 — top-priority justification):* this arc CURES the team's own stop-problem. The agents kept stopping ("why are we stopped?" ×3, trust-eroding) because, when a task is blocked/done, NOTHING pushes the next work — we run poll/relay/hold, not push. The cure = agents SUBSCRIBE to events (the store's EXISTING subscribe/wait_for_event/dispatch + role-filter, ~60-70% built) → pushed the next-work (deliver→acceptor · CI→gated-role · reject-readable · broadcast). The missing push-signals ARE this arc's deliverable; the product cures our own pain. *RECEIVE-MODEL:* Ryan's "you should be able to subscribe to events" (2026-06-06) points the receive-model at subscribe/wait_for_event (the Epic-body's grounded vision) over #1656's PTY-injection — pending owner-confirm whether #1656's broadcast reshapes onto subscribe (the injection-guard / \r-submit / security-verdict RETIRE). That confirm resolves the size-gate for slices 1-3 + MCP→CLI. *Design-surface ready:* `docs/design/channels-independence/cli-surface-grammar.md` (outgoing half) + forms-grammar serves the riding #1518. **Aquila-retro 2026-06-07** folded affordance-candidates into this arc: #1631 passive-push (design-of-record banked — the EVENT-tier cure for the stop-problem) + the computed-style/DOM-inspect tool (render-verification Feature, all-3-seats-named) + #1593 visibility; see the 4-axis master-canon lesson + `docs/bpmux-delivery-mental-model.md`.
2. **N+2 — design consistent + accessible BY CONSTRUCTION** [Epic #1659 Darcula]. *Outcome:* one design-system — every component shares the grammar (tokens → primitives → application), WCAG-baked per-slice, no per-surface drift. Epic sequences it AFTER Claude-independence. Designer mockup-first (foundation/primitives/forms firming; list/row + application next). *IMPACT-FLAG (data-driven, for IPM/Ryan steer):* the felt-VERIFICATION pain — real-OS-input-driving to gate native-DnD felt-behaviors Playwright can't (it CAUSED Aquila's 4 owner-rejects) — is high-impact and may warrant elevating ABOVE the design-system; weigh at the N+2 IPM.
3. **N+3+ — gate-engine + team-verification/infra**. *Outcome:* the team ships without flaky-gate friction + verifies what it builds. Tucana #1545 (gate-engine/canon-tooling) + the impact-high team-pains via solution-brainstorming (HMW, cross-disciplinary): per-vertical push-gating (flaked 5+× — false-red erodes signal-trust) · real-OS-input felt-verification · mockup-present-to-owner + semantic-view-nav agent-tooling.

Supporting detail (mechanism/slices beneath the outcomes above):

- **Darcula design-system — Epic [#1659](https://github.com/RyanDur/bpmux/issues/1659)** — PLANNED 2026-06-06 into **3 build-ordered iterations** (sequence AFTER the channels-independence work / Epic #1657), per designer's firsthand-grounded **7-slice decomposition** (the 40-`.tsx` visible-component inventory; `docs/design/design-system/cross-cutting-research.md`): **Fornax #1678** (FOUNDATION slice-0: palette · grid+density · typography · motion+focus-ring + the cross-cutting TOKEN-values — breakpoints/target-size/focus-ring; + STATUS-GRAMMAR slice-1: the one 5-state `●◌◐✗○` across CiGlyph/SyncStatusGlyph/online-dots) → **Antlia #1679** (PRIMITIVES: slice-2 button+modal+form-controls · slice-3 list/row — SortableList/StoryRow/MemberList/Iteration/DragGrip, the #1524/#1517 design-home) → **Horologium #1681** (APPLICATION: slice-4 board · slice-5 chat · slice-6 sidebar+metrics+appshell-chrome + overall-look #1666 + a thin **audit-sweep #1682** last). **Cross-cutting is BAKED into each slice's acceptance, NOT separate stories** (designer-grounded: a11y/keyboard/responsive are PROPERTIES of each component — tokens in Fornax, application baked per-slice, audit-sweep verifies coverage). **RETIRED:** Mensa #1680 (standalone cross-cutting iteration) + #1670/#1671/#1672 (cross-cutting-as-stories — dissolved into per-slice acceptance-clauses). ~12 slice-stories; Board(4)/Chat(5) are component-heavy and MAY split into their own iteration(s), sized when their mockups firm (PM sizing-call → would re-sequence in place). designer-led mockup-first (slices 0/1/2 mockups done + WCAG-audited; 3 next), Ryan reviews taste. Mockups: `docs/design/design-system/{button-modal-research.md, darcula-foundation-mockup.html, button-modal-primitives-mockup.html, cross-cutting-research.md}`. #1518's reject-modal rides Aquila on current tokens + re-skins when Antlia's modal-primitive lands. NOTIONAL — designer firms each mockup-first; per-slice children retitle as their mockup grounds.
- **board-IA / grounding arc** (registered 2026-06-05) — REALIZED 2026-06-07 as two epics: **Epic [#1702](https://github.com/RyanDur/bpmux/issues/1702) Roadmap view** (#1703 readiness-substrate → #1704 view → #1684/#1685 reach → #1705 nudge) + **Epic [#1706](https://github.com/RyanDur/bpmux/issues/1706) Rot-free grounding** (#1697 fresh-briefing + #1683 scoped-read [done]). **PLACEMENT — Ryan chose B 2026-06-07 ("the affordance to the users is more important" → roadmap-visibility SOON over build-once-later):** the roadmap-instrument is its OWN near-term iteration **Crux #1707** (right after Lupus), the whole chain self-contained (substrate→view→reach→nudge), built on CURRENT tokens; ONE bounded board-primitives reskin when Darcula's Horologium lands (the #1518 pattern — view auto-rides token-swaps, only board-primitives restyle; designer-tracked for Horologium). #1697 → Epic #1706 (placed at a future IPM). All stories value-checked (Ryan-directive) + designer pre-pickup-reviewed. Sequence: Lupus → **Crux** → Darcula → Tucana → Hydra. (Earlier Crux-retire + the Tucana/Horologium blend were superseded by Ryan's B call.)
- **Tucana [#1545](https://github.com/RyanDur/bpmux/issues/1545)** — Gate-engine + canon-tooling (#1541/#1542/#1503-04/#1506/#1514/#1505)
- ⚠️ **Reticulum #1464** — listed in prior snapshots but the entity is NOT FOUND in the board (stale ref — recreate if still wanted, else drop). Pyxis #1521 is now the CURRENT iteration above (its old Rooms-Track-B/C/D theme was superseded by the channels-independence reframe), no longer a horizon item.
- **Octans** — In the pre-Carina documented order (Pyxis-body 2026-05-22); entity not yet created

### Iteration-ceremony canon (process notes)

- **IPM (planning)** — team-autonomous on team-data (dev relative-complexity + designer vertical-check + PM velocity-bound + drain-rate). Per `decide-dont-ratify` skill: PM commits + opens; doesn't ratify-ask Ryan on delegated HOW.
- **Open-flow** — team IPM-commits → PM designates current + promotes-to-ready (briefing-first + dependency-correct, WIP-1) → PM signals Ryan for **start-clearance** (owner-axis gate) → Ryan clears → dev pulls top-down.
- **Mid-iteration** — team-autonomous (dev/designer drive per WIP-1 + desk-check + chore-triad).
- **Close** — terminal per #1556 (close promotes nothing; members reach ready via the next iteration's open/designate).
- **Acceptance** — at queue-drain, Ryan-call drives `run-acceptance` (owner-walk on owner-class stories + PM-fires team-class with fresh designer+dev verdicts).
- **Story-slice rule** (Ryan 2026-05-27): any ≥3pt story splits to ≤2pt vertical slices (capability end-to-end through all layers — never horizontal foundation+wiring cut).
- **Multiple iterations** — split-iterations run SERIAL (one-after-another), never parallel.

Pipeline updated when each iteration closes + its successor's IPM settles.

---

## Horizon 1: Polish the Dogfood

**Goal:** A developer who is not Ryan can use bpmux solo and have a good first experience.

**Timeline:** 8 weeks from start.

**Gate:** /help command works, no major TUI bugs, README explains the conduit model clearly.

### Milestones

1. **TUI reliability** ✅ -- borders, input, auto-scroll, message truncation all shipped
   - #14–#18 all closed

2. **Awareness** ✅ -- visual identity, notifications, unread counts, directed messaging, status dots
   - #19–#23 all closed. Status dots (#59, #67) shipped. Unread badge (#63) shipped.

3. **Onboarding** ✅ -- help overlay (#60), command input bar (#64), message history (#24)
   - #24–#26 all closed

4. **Session awareness** ✅ -- participant roster (#27), join/leave (#28)
   - #27–#28 all closed

5. **Transport reliability** ✅ -- cursor reads (#29), reconnection (#30, #73), system messages (#31)
   - #29–#31 all closed. Session handoff (#73) shipped.

6. **Channel navigation** ✅ -- sidebar tree view (#66, #71), /board (#69), channel switching (#54, #56)
   - All closed. Sidebar with ⌥1 toggle, three-state focus, unread counts.

7. **Message rendering** ✅ -- markdown (#80), code blocks (#61), syntax highlighting
   - tui-markdown integration with bold, headers (no hashes), syntax-highlighted code blocks.

### Active work (Horizon 1 polish)

- [#92](https://github.com/RyanDur/bpmux/issues/92) Channel awareness — agents know which channel the owner is talking in (verifying filter behavior)
- [#97](https://github.com/RyanDur/bpmux/issues/97) Bridge filtering — agents only receive relevant messages (token savings)
- [#98](https://github.com/RyanDur/bpmux/issues/98) Focus indicators for active panel
- [#100](https://github.com/RyanDur/bpmux/issues/100) Contextual shortcuts in status bar
- [#101](https://github.com/RyanDur/bpmux/issues/101) Type-to-filter channels in sidebar

**Recently shipped:** #79 (channel view), #81 (paste fix), #68 (status dots survive restart), #65 (status dots), #83 (/roadmap), #84 (clean startup), #85 (mouse clicks), #86 (text wrap alignment), #87 (activity filtering), #88 (large markdown), #90 (word wrap), #91 (broadcast banner), #93 (remove handoffs), #94 (remove pins), #95 (remove MCP endpoint), #96 (drop connected count)

**Research backing:** Edmondson (1999) -- first interactions disproportionately determine whether a tool feels safe or threatening. Gutwin & Greenberg (2002) -- minimum awareness requires presence, identity, and action visibility. Clark & Brennan (1991) -- reviewability and cotemporality are the constraints that matter most for text media.

**Resolved risks:**
- ~~Markdown rendering is a rabbit hole.~~ Solved with tui-markdown + syntect. One dependency, no custom parser.
- ~~Readline editing is non-trivial in terminal raw mode.~~ Shipped with crossterm + custom input handling.
- ~~Agent activity indicator requires a lightweight status message type.~~ Shipped via hooks (PreToolUse → busy, Stop → available).

---

## Horizon 1.5: Public Face and Memory

**Goal:** bpmux has a website, a migration story, and agents can rebuild theory on reconnect.

### Milestones

1. **Website/docs app** -- explains what bpmux is and why, surfaces research, provides OS-specific downloads
   - Reference pattern: battleship repo (React + Vite + GitHub Pages + GitHub Releases)
   - Not prescriptive — shares what we do and what worked, not a mandated process

2. **Arcs — walkable reasoning paths** -- the log captures teaching moments; arcs make them findable
   - Naur's theory building: theory can't be copied, only rebuilt through walkthrough. Arcs are asynchronous walkthroughs.
   - Experiments showed: log-only onboarding scores 4.5/5 on culture, one arc raises culture by a full point
   - Research backing: CBR (Schank), Experience Factory (Basili), conversation analysis (Sacks), Bohm's dialogue
   - Nature (base model) + nurture (arcs) = judgment. Neither alone is sufficient.
   - First real test: machine migration 2026-04-11

3. **Project health signals** -- glanceable board with Accelerate/DORA-aligned metrics
   - Throughput (stories accepted/cycle), cycle time (start to acceptance), rejection rate
   - Reference: Forsgren, Humble, Kim — measure flow, not output

---

## Horizon 2: Connect the Tools

**Goal:** Two developers using different coding agents collaborate through bpmux.

**Timeline:** 4-6 months after H1.

**Gate:** WebRTC peer connection working, at least 2 different AI tools connected, participants distinguishable by identity.

### Milestones

1. **P2P connectivity** -- real networking, not just localhost
   - [#5](https://github.com/RyanDur/bpmux/issues/5) Two developers can connect and chat in real-time (WebRTC)

2. **Identity and trust** -- who is who, and can you trust them
   - [#8](https://github.com/RyanDur/bpmux/issues/8) A developer knows who to trust in the chat
   - [#34](https://github.com/RyanDur/bpmux/issues/34) Participants identified by name and tool, not hardcoded suffixes (icebox -- see below)

3. **Multi-tool integration** -- any coding agent joins with one command
   - [#32](https://github.com/RyanDur/bpmux/issues/32) A developer can connect any coding agent to bpmux with one command (icebox -- see below)
   - [#33](https://github.com/RyanDur/bpmux/issues/33) A developer using a local MLX model can participate (icebox -- see below)

4. **Context management** -- sessions that scale
   - [#7](https://github.com/RyanDur/bpmux/issues/7) An AI respects the context budget

**Research backing:** Clark & Brennan (1991) -- cross-tool grounding costs are determined by the least-capable participant's constraint profile. ChatCollab (Klieger et al., 2024) -- validates shared-channel architecture under homogeneous conditions; heterogeneous conditions are uncharted. Cross-tool integration research confirms the lowest-common-denominator problem: a heterogeneous medium converges to its most limited participant unless the protocol compensates.

**Risks:**
- **Channels dependency.** Claude Code channels are experimental with no stability guarantee. The hook-based fallback must remain the reliable baseline. WebRTC (#5) is the path to real P2P that does not depend on Claude Code experimental APIs.
- **Heterogeneous grounding costs.** Different tools have different constraint profiles (push vs. poll, hooks vs. MCP-only). No research covers this scenario. bpmux will generate its own evidence.
- **Multi-human sessions are unvalidated.** No research supports mixed human-AI teams of 6-8 in flat chat. Build the minimum (roster, tokens, snapshots), run real sessions, let data decide.

---

## Horizon 3: Scale the Platform

**Goal:** Teams adopt bpmux with governance and compliance.

**Timeline:** 9+ months. Directional, not planned.

**Gate:** SSO or equivalent auth, audit trail export, `brew install bpmux`.

### Milestones

1. **Distribution and reliability**
   - [#9](https://github.com/RyanDur/bpmux/issues/9) bpmux deploys reliably (CI/CD, cross-platform builds, release automation)

2. **Security and compliance**
   - [#6](https://github.com/RyanDur/bpmux/issues/6) A developer's conversation is secure (DTLS, at-rest encryption, session tokens)
   - JSONL log as compliance infrastructure (EU AI Act 2026, audit trails)

3. **Accessibility**
   - [#3](https://github.com/RyanDur/bpmux/issues/3) A developer can see the chat in their browser (read-only web UI)
   - [#4](https://github.com/RyanDur/bpmux/issues/4) An AI can start a session without the developer touching a terminal

4. **Platform evolution**
   - A2A protocol adoption if 3+ major AI coding tools support it in production by H2 ship date
   - Session recordings as searchable knowledge base
   - Self-hosted-first with managed service option if demand appears

**Research backing:** EU AI Act enters enforcement 2026, high-risk agent systems require continuous audit trails, penalties up to 30M EUR. Enterprise AI collaboration market projected $8.5B (2026) to $35B. A2A protocol ecosystem stabilizing (Linux Foundation, Apache 2.0, co-founded by OpenAI/Anthropic/Google/Microsoft).

**Risks:**
- **Self-hosted vs. SaaS tradeoff.** Self-hosted limits adoption (GitHub vs GitLab, Slack vs Mattermost). Current architecture supports both. Do not pre-build managed infrastructure.
- **No enterprise buyer identified.** Gap between "captures the right data" and "meets procurement requirements" is enormous.
- **A2A is a delegation protocol, not a participation protocol.** It solves discovery, not group conversation. Useful for agent discovery only; bpmux's own WebSocket + MCP stack handles collaboration.

---

## Icebox

Issues that need team discussion before scheduling.

| Issue | What it is | What we need to decide |
|---|---|---|
| [#32](https://github.com/RyanDur/bpmux/issues/32) Connect any coding agent with one command | Generic bridge setup for Cursor, Copilot, etc. | What "one command" means when each tool has different MCP integration models |
| [#33](https://github.com/RyanDur/bpmux/issues/33) Local MLX model participates | Non-cloud AI joins a session | Whether the bridge contract is general enough or needs a new adapter pattern |
| [#34](https://github.com/RyanDur/bpmux/issues/34) Identity by name and tool, not hardcoded suffixes | Dynamic participant identity | Whether identity is session-scoped (H2) or persistent (H3), and what the message schema change looks like |
| [#72](https://github.com/RyanDur/bpmux/issues/72) bpmux-owned desktop notifications | Replace code-notify with native bpmux notifications | Focus detection (crossterm FocusGained/FocusLost) and OS notification integration |
| [#75](https://github.com/RyanDur/bpmux/issues/75) / [#76](https://github.com/RyanDur/bpmux/issues/76) WebRTC P2P | Spike + full story for peer-to-peer messaging | Rust library choice (datachannel, str0m, webrtc crate), battleship has Kotlin/Spring prior art |
| [#48](https://github.com/RyanDur/bpmux/issues/48) Auto-onboarding for AI | AI gets role + project context from session on join | Whether this is bpmux's job or the workspace CLAUDE.md |

---

## What We Will Not Build

| Anti-feature | Reason |
|---|---|
| **Threading** | Flat chat preserves shared awareness for small teams (Gutwin & Greenberg). Threading fragments it. Re-evaluate only if flat chat breaks at 6+ participants in H2. |
| **Real-time collaborative editing** | bpmux is a communication channel, not a shared artifact editor. Single-keyboard constraint is a feature (Zuill, 2014). |
| **Agent runtime / orchestration** | bpmux is infrastructure, not an orchestrator. Every framework (CrewAI, LangGraph, AutoGen) is an orchestrator. bpmux carries messages. |
| **Project management features** | Backlog lives in GitHub Issues. Workflow lives in process docs. |
| **File sharing, voice/video, emoji reactions** | General-purpose chat features dilute the product. |
| **Mobile client** | Users are developers at workstations. |
| **Persistent user accounts** | Session-scoped identity through H2. Accounts only if a paying enterprise customer needs them. |
| **Multi-pane layouts beyond sidebar** | Sidebar is the navigation surface. No additional panes — keep the TUI focused. |
| **Plugin/extension system** | Not now. But the bridge contract should not prevent future extensibility. |
