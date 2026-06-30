# Story shape — the team's shared work-unit canon

The SHAPE of a story is team-wide information, not one role's private concern. Every
role consumes it:

- **PM** authors Features/Bugs with it.
- **dev** builds from the acceptance-criteria (and authors Chores).
- **designer** desk-checks + pre-pickup-reviews against it.
- **owner (Ryan)** accepts via who-it's-for + the expected outcome.

This doc is the ONE shared source for the FORMAT. Per-role skills hold the per-role
APPLICATION (PM's filing checks, designer's review-coverage, dev's build-from-ACs) and
**cite this doc** for the format — they must not re-assert (or diverge from) it. A
divergent second-source is the failure mode this doc exists to prevent.

Load with the rest of the process-canon chain (memory-architecture / iteration-ceremony-cycle
/ process-state-machine / handoff-process).

---

## 1. The Use Case — who it's for, what, why (Mike Cohn)

Every Feature/Bug leads with:

> **As a** [specific persona] — **I want to** [action] — **so that** [outcome].

- **"As a [persona]"** is load-bearing TWICE: it names *who the story is for* (the
  beneficiary) AND it drives **acceptor-class** (§3). Drop it and both who-it's-for and
  who-accepts go ambiguous. Use a specific persona ("As Ryan supervising agents", "As an
  agent-in-room"), never a generic "As a user".
- **"I want to [action]"** is what the persona DOES, not a feature you're building.
- **"so that [outcome]"** is the motivation — NOT a restatement of the action.

## 2. Acceptance Criteria — Gherkin (the pass-shape)

Concrete, testable scenarios:

> **Scenario:** [human-readable value]
> **Given** [precondition] — **and Given** [more] — **When** [trigger] — **Then** [outcome]

- **One `When` + one `Then` per scenario.** Multiple Whens/Thens = the story is too big →
  split it (each When/Then pair is its own story candidate).
- Multiple `Given`s are fine (preconditions stack).
- `When` aligns with "I want to"; `Then` aligns with "so that".
- `Then` must be observable/measurable — never "Then it's better/faster".

## 3. The observation-falsifier — the reject-shape (PAIRS with Gherkin, does not replace it)

> "I would reject if I observed X."

The falsifier frames acceptance as observation-that-FAILS; the Gherkin ACs are the
pass-shape. Both belong in the body. A story with only a falsifier (no Gherkin ACs) does
NOT yet follow this canon — and neither does one with only Gherkin (no falsifier). The
falsifier is the AC's `Then` written as a rejection.

## 4. Who-it's-for → acceptor-class (beneficiary ≠ walker)

The **As-a persona** names the BENEFICIARY. The **acceptor-class label** names the WALKER —
who fires acceptance. They can differ:

- **owner-walk** (`awaiting owner`): a user-do affordance Ryan observes ("As Ryan… I click X
  and see Y"). Ryan walks + accepts.
- **team-verify** (`awaiting team`): substrate/reliability behavior the three agents verify
  deterministically (agents are users too — an agent-facing capability still routes here when
  its value is agent-side).
- **chore** (`awaiting dev`): behaviour-preserving substrate; dev accepts.

Example of the split: a story whose persona is "As an agent" but whose observable outcome is
"Ryan sees the routed message in the broadcast-feed" is acceptor `awaiting owner` — the As-a
names the beneficiary (the agent), the acceptor names the walker (Ryan). Make the label match
WHO WALKS, and state it explicitly.

## 5. Filing-class — Feature / Bug / Chore (ground in the SHAPE of the work, not the title)

- **Feature** (PM-authored): adds NEW capability / visible value an end-user OR an agent-user
  can OBSERVE.
- **Bug** (PM-authored): a PRODUCTION issue that affects the EXPECTED USER EXPERIENCE — wrong
  *observable* behaviour in a feature ALREADY delivered AND accepted, measured against an agreed
  requirement (Ryan-canon 2026-06-22: "bugs are production issues that affect the expected user
  experience"). The discriminator is **user-experience impact, NOT incident-provenance**: a past
  outage/incident does not make its internal proper-fix a Bug. Missing functionality, absent
  polish, a change-in-behaviour, or internal architecture/infra/tooling/storage/test-work with
  no user-experience change is NOT a bug → file a Feature (new observable value) or a Chore
  (internal, behaviour-preserving). Verify-against-requirement BEFORE logging (is it actually
  wrong vs an agreed requirement the USER experiences, or just not-yet-built / internal?). See
  §5a for the bug-shape + the no-point rule.
- **Chore** (dev-authored + dev-accepted): maintenance / refactor / tooling / config / storage /
  test-infra that PRESERVES the user-experience — no new capability, no visible change. A chore
  MAKES DEVELOPMENT BETTER. (An internal fix prompted by an incident is still a Chore if the user
  experiences nothing different.)

**The decisive question per type** (type by the SHAPE of the work + user-experience impact —
NEVER by the title, NEVER by the incident that prompted it):

1. **Does it change what an end-user OR agent-user OBSERVES?** — NO → it's internal → **Chore**
   (dev-authored + dev-accepted). Stop. Do not call it a Bug because an incident prompted it.
2. If YES — **is the observable behaviour NEW, or is an ALREADY-accepted feature behaving WRONG
   vs an agreed requirement?** — NEW value → **Feature** (PM-authored, owner-raised); WRONG
   behaviour → **Bug** (PM-authored, unpointed, §5a shape).

> **The trap (#1803, 2026-06-22):** the SQLite storage cutover was filed `[Bug]` because it
> traced to the startup-outage incident. But the user never experiences the storage backend (the
> symptom was already stop-gapped) → no user-experience impact → it was internal infra (Chore-class /
> owner-led work), never a Bug. **Incident-provenance is not a type-signal; user-experience impact is.**

Two disguises this canon rejects at FILING-time (catch here, not at pickup):

- **Masked-horizontal:** a "Chore" that ADDS behaviour which is merely *not-yet-user-visible*
  (a backend layer / unwired capability toward a feature) is hidden feature-work → reclassify
  to a vertical **Feature** that delivers user-observable value. Feature-substrate RIDES the
  vertical slice that uses it; it is never carved off as a standalone backend chore. If you see
  this, **tell the author/dev directly.** (Ryan-canon 2026-05-29: "a chore should make
  development better, not be masked feature work… a way to hide horizontal development.")
- **Diagnosis-as-fix → SPIKE:** a symptom-remediation framing ("stabilize/fix/de-flake X",
  esp. a PLURAL symptom "flakes/races/clusters") whose distinct ROOT CAUSES can't be named
  without investigating is a SPIKE, not a fix-slice. Site-enumerability ≠ root-cause-
  enumerability. File the spike; let it surface the real fix-verticals.

### 5a. Bug-shape + the no-point rule (Pivotal Labs — Ryan-canon 2026-06-02)

A bug story is the PROMPT for a developer conversation (a pairing), not a substitute for one —
keep it clear, unpointed, and tied to a defect in an already-accepted feature. It still names
who-it's-for (acceptor-class §4 — usually `awaiting owner` since the defect is in a user-/owner-
facing feature). Its body carries four sections (the Description's *what-should-happen* + the
numbered Steps-to-Reproduce serve the pass/falsifier role §2–3 for defects):

1. **Title** — short, actionable, answers what + where. Format `[Component/Page] - [brief problem]`
   (e.g. "Checkout - Submit button spins indefinitely on mobile Safari").
2. **Description** — *what is happening* (the incorrect behaviour) vs *what should happen* (the
   expected outcome / the agreed requirement it violates).
3. **Steps to Reproduce** — the MOST critical part: exact NUMBERED actions that trigger it
   (1. go to X · 2. as [persona] · 3. do A · 4. do B), noting the result after a step when it
   narrows the cause.
4. **Resources** — visual/technical proof: screenshots / short recordings + browser / OS / device /
   console / network logs.

**No points on bugs (or chores).** Bugs are unpredictable and add no new user value; pointing
them distorts velocity and invites dwelling rather than fixing — leave them unpointed.

## 6. Domain-language (no impl-leak in the contract)

The Use Case + Gherkin + falsifier (the CONTRACT) speak the DOMAIN — what the
user/owner/agent observes. Storage-serialization (GitHub labels), typed field-names, and
code-structure (file:line, function-names) are transformed-at-the-boundary and quarantined
to a separate `## Dev-grounding` section — NEVER in the contract. Leaking impl into the
contract DRIVES a specific implementation instead of describing the need.

## 7. Vertical slices (each slice ships observable value)

Decompose by CAPABILITY (vertical), not by LAYER (horizontal). Each slice — including a
thin-then-enrich first slice — must ship something the beneficiary can OBSERVE and that can
be desk-checked against a falsifier. A backend-only slice that ships nothing observable is a
horizontal cut and is rejected.

---

## Provenance

PM-seeded from the `user-story` + `pre-filing-audit` skills (Mike Cohn *User Stories Applied*
+ Gherkin/BDD). Lifted to this shared home 2026-05-29 on Ryan's "this is important
information for everybody" + designer's "one source — cite, don't re-assert". Consuming
skills (PM `user-story` / `pre-filing-audit`; designer `pre-pickup-review`) cite this doc for
the FORMAT and keep their own APPLICATION local.
