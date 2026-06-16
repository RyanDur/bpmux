# Roster membership model — connected → represented

Designer 2026-06-14, for the #1721 rework (the membership-axis the re-key
refactor regressed). PM writes the falsifier against this model.

**Owner-canon (Ryan 2026-06-14, verbatim):** "all the agents should show up in
the roster whether they have a role or not. your role name is just a display and
conversational affectation. you are connected, you should be represented in the
roster." + "you are supposed to … key off your ids … your roles are for display."

## The two layers the refactor conflated

The roster is the team's presence-surface — *who is here, and how available*. It
has **two orthogonal layers**. The #1721 refactor fixed layer 2 and regressed
layer 1.

### Layer 1 — MEMBERSHIP (who appears) = CONNECTEDNESS

- **Any connected agent appears**, automatically. Connectedness — a live
  routing-identity/session (and/or a detected live terminal) — IS the trigger to
  appear. **No manual `bpmux register` precondition. No role-gate.**
- **Role-less agents appear too.** Connectedness is the gate, not having a role.
- **Keyed by the stable routing-id** (session-id today; a durable
  directory-independent agent-id is the future substrate fix). Never keyed by the
  mutable role display-name.
- **Role = a display LABEL on the row** (Designer / Dev / Pm / Owner) — a
  display/conversational affectation, never the gate for appearing. The display is
  what the agent **sets** via register; until it sets one, the row shows the
  **dir-name default** (the workspace dir basename, e.g. `bpmux-designer` —
  `agent_detect.rs:111`). The default holds the row's label until the set-display
  replaces it. (The `agent-<id>` string from `bridge_agent_name` is an internal
  leave-key, **never** the display default.)
- **render = f(substrate):** the substrate is CONNECTEDNESS; the roster renders
  it. Hiding a connected agent is an **absence-fabrication** — the same
  honest-default violation as the all-offline bug, one layer up (offline-fabrication
  → absence-fabrication). The surface must not lie about who is here.

### Layer 2 — PRESENCE (the dot per row) = terminal-on-ness

This is #1721's dot-color derivation, which **works** (verified: designer + dev
render busy off the live terminal; all-offline cured). It STANDS.

- busy = terminal active < 7s (incl thinking — thinking IS the output stream)
- idle = quiet ≥ 7s (the confident green, free-to-talk)
- need = owner-park
- offline = genuinely disconnected (the roster-leave path)
- A connected member whose terminal isn't resolved yet = **present** (not offline,
  not a false-green) until it resolves.

## The regression (#1721 @ 5f6f9743) and the fix

**Regression:** the re-key path broke the connected→represented auto-populate —
connected agents vanish from DIRECT until a manual `bpmux register`. Grounded:
`useTeamRoster.ts` changed only the Away→Idle rename; `activity_presence.rs` only
*updates* presence on existing roster members (it `continue`s on unresolvable
ones — it never *populates*); the `RosterStore` is upsert-only with no
default-seed. So the membership that "used to be present" came from the
auto-detection/registration populate path, which the re-key broke. **"Re-register"
is the symptom, not the fix.**

**Masking:** the e2e pre-seeded the roster (`seedRoster: ['dev:claude-code']`) — a
test that seeds the precondition cannot catch a regression *in* that precondition.

**Fix (rework):** membership populates from CONNECTEDNESS, not from manual
register. A running/connected agent auto-appears.

## The duplicate-row repro (Ryan 2026-06-14) — the same name-keying disease

After 78259464 (auto-appear shipped), Ryan's roster showed **four rows for three
agents**: the dir-name defaults (`bpmux-designer`/`-dev`/`-pm`, gray) *plus* a
separate `designer` (busy) once that agent ran `bpmux register`. Grounded
firsthand — one agent gets **three different names**, and the store matches rows
**by name**, so no cleanup path can find what another path created:

- auto-detect **joins** by `dir_session.name` → `"bpmux-designer"` (dir basename) — `agent_detect.rs:111`
- register/death **leave** by `bridge_agent_name(terminal_id)` → `"agent-<8char>"` — `routing_identity/store.rs:10`
- register **joins** by `payload.display` → `"designer"` (role) — `register.rs:42`

So `bpmux register` joins `"designer"`, tries to leave `"agent-xxxx"` + previous
display — neither matches the live `"bpmux-designer"` auto-row → it orphans → two
rows. This is **the same name-keying disease** as the all-offline + membership
bugs, one layer over. The id-key cure collapses it: register finds the row by
`routing_id` and renames the display in place — and the leave-by-mismatched-name
dance gets **deleted**, not patched. (Patching the leave to match the basename
would be "making a bug" — Ryan-canon: fix what you broke, key by id.)

**Precedence rule the cure must encode** (not just "key by id"). Owner-canon
(Ryan 2026-06-14, verbatim): *"the name you set yourself is your DISPLAY name, the
other is the default until you set your display name. that is the reason for you
setting your display name."* So the dir-basename is a **default placeholder** that
holds the (id-keyed) row until the agent sets a display name; `register` sets the
real display name, which **replaces** the default *on that same row*. There is one
row throughout — the label goes `default → set`. Two writers touch the display
(`register` explicit, `agent_detect` default), so: **the set display-name WINS;
the auto-detect default only fills the row's label on first insert and never
clobbers a set name back to the basename on a re-detect tick.**

## Falsifier shape (PM authors against this)

- A connected agent appears in the roster with **no manual `bpmux register`**.
- A **role-less** connected agent still appears, keyed by id, **labeled by the
  dir-name default** (`bpmux-<dir>`) until it sets a display name via register.
- The row is keyed by the routing-id, **including the store match-key** — never the name.
- **One row per connected agent.** Auto-detect default + `bpmux register` collapse
  to a single row (no `bpmux-<dir>` / role duplicate) — pre-cure repro = Ryan's
  2026-06-14 screenshot; post-cure = a single `designer` (busy) row, no orphan.
- **The role-name persists across an `agent_detect` re-detect tick** — re-detect
  upserts the existing id-row without clobbering the registered display back to the
  basename.
- The dot resolves per-member from terminal-on-ness (layer 2, unchanged).
- **The e2e must NOT pre-seed the roster** — the seed is what hid this regression;
  the test must exercise the auto-populate from a real connect.

## Provenance

Same identity-rule as the routing-key (key off the id, role is display), applied
to the roster render. See `design_decisions_log.md` (the dot entry) +
`[[feedback_operate_off_mcp_via_cli_key_off_id]]`. Dot-color model + #1732
supersession stand; this is the membership axis, separate from the derivation.
