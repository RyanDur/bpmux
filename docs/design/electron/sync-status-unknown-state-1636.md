# SyncStatusGlyph — unknown/loading visual state (#1636)

Owner-pick mockup. Ryan said he wants "to know more" about the
SyncGlyph false-green bug; this is the design half he gates.

## The bug (grounded)

`SyncStatusGlyph.tsx:23` — `if (!report) return 'connected'`. When the
sync-report is null, the glyph renders the all-clear **green ●
"connected"** state. The actual error surfaces ONLY in the hover
tooltip. So the at-a-glance signal AND the screen-reader aria-label
both say "all clear" while sync has errored or hasn't loaded.

`report` is `null` in four cases (`syncStatusService.ts:79-107`):
| # | case | benign? |
|---|---|---|
| 1 | pre-first-load (initial state, fetch not yet returned) | benign / transient |
| 2 | status fetch failed (HTTP not-ok) | trouble |
| 3 | network error (caught exception) | trouble |
| 4 | no `apiBase` provided | misconfig / sustained |

The snapshot carries `{ report, error }` — so the component CAN tell a
**benign pre-load** (`report=null, error=null`) apart from a
**sustained/failed read** (`report=null, error≠null`). That distinction
is the design fork below.

## 3-field spec header

**user-job:** When Ryan glances at the sync glyph (or hears its
screen-reader label), he needs the signal to NEVER read "healthy/
connected" unless a successful sync report actually confirms it —
because a false all-clear hides real trouble exactly when he'd want
to act.

**observation-falsifier:** I would reject if a null/errored sync-report
renders the green "connected" glyph or a "connected" aria-label; OR if
the only place the trouble shows is the hover tooltip (glance + SR
paths must already carry it); OR if the new state is indistinguishable
by shape from an existing state (color-alone fails colorblind glance).

**team-shape-independence:** state derives purely from
`{report, error}` — no role list, no agent-composition assumption.

## Principle alignments

- **shape + color, not color-alone** (phase-1a a11y): the new state(s)
  must be a distinct GLYPH, not just a recolor — colorblind-readable
  at a glance.
- **contrast threshold — this is a GRAPHIC, not text:** SyncStatusGlyph
  renders a glyph (`<i>` char), so WCAG **1.4.11 non-text 3:1** applies,
  NOT the 4.5:1 text threshold. Each option's color must clear 3:1
  against the glyph's actual background (verify at desk-check via
  a11y-audit). NB: the existing connected-green (#16a34a, light) is
  3.29:1 — PASSES 3:1 as a glyph. Do NOT over-correct it to 4.5:1.
  (The separate `.titlebar-app-state` "online" TEXT in the same green
  DOES fail 4.5:1 — that's a different element, not this story; see the
  dev a11y-debt flag.)
- **honest-at-a-glance** (this bug's whole point): glance + aria carry
  the signal; tooltip is detail-on-demand, never the only carrier.
- **correctness ceiling — don't over-claim:** an unreachable *status*
  is INDETERMINATE, not a confirmed sync failure (drained/startup-fail
  derive from a *populated* report). So the unknown state's alarm
  ceiling is **amber/warning, never red.** Red would assert a failure
  we haven't observed.

## The existing 7 states (so the new one is distinct)

| state | glyph | color token | reads as |
|---|---|---|---|
| connected | `●` | `--accent-success` (green) | healthy |
| syncing | `◌` | `--fg-secondary` (gray) | in progress |
| offline-queued | `⚠` | `--accent-warning` (amber) | queued, offline |
| drained | `✗` | `--accent-error` (red) | dropped (permanent) |
| recovering | `↻` | `--accent-warning` (amber) | rehydrating |
| stale | `◔` | `--accent-warning` (amber) | past staleness |
| startup-fail | `⊘` | `--accent-error` (red) | startup pull failed |

Taken glyphs: `● ◌ ⚠ ✗ ↻ ◔ ⊘`. Taken colors: green / gray / amber / red.
`--accent-info` (cyan) is defined but **unused** in sync states.

## The fork: ONE unknown state, or TWO (loading vs unreachable)?

The bug is fixed the moment a null report stops rendering green. The
open call is whether to ALSO split the benign pre-load blip from a
sustained "can't read sync status" — they're different operator
meanings, and the substrate (`snapshot.error`) supports the split.

---

### Option A — one honest "unknown" state (simplest correct fix)

One state for all four null cases.

```
  ○   Sync state: unknown — no successful sync report yet
  └ glyph: ○ (hollow circle, U+25CB)
    color: --accent-info (cyan)
    aria : "Sync state: unknown — no successful sync report"
    tip  : surfaces snapshot.error if present
```

- Fixes the bug: never green when unconfirmed. `○` (empty) vs `●`
  (filled) is an honest "not-yet-confirmed" contrast.
- Simplest: one new state, one glyph, one aria.
- Trade: a sustained fetch-failure reads the same neutral cyan as a
  benign sub-second pre-load — detail only in tooltip.

### Option B — two states: loading vs unreachable (fuller fidelity) ★ designer-lean

Discriminate on `snapshot.error`.

```
  ○   (report=null, error=null)  → loading
  └ color: --fg-dim (dim gray)   aria: "Sync state: loading — awaiting first report"
        benign, transient: empty ring "filling up"  (○ → ◌ → ●)

  ?   (report=null, error≠null)  → unreachable
  └ color: --accent-warning (amber)  aria: "Sync state: unknown — sync status unreachable"
        sustained trouble: distinct glyph + amber (NOT red — status indeterminate, not confirmed-failed)
```

- Distinct GLYPH per state (`○` vs `?`) AND distinct color →
  colorblind-safe, satisfies shape+color.
- A sustained failure escalates off the benign "loading" read — closes
  the "trouble hid as loading" residue of the original bug.
- Trade: two new states instead of one.

### Option C — one glyph, color-shifts on error (compact)

```
  ○   color = --fg-dim while error=null (loading), → --accent-warning if error≠null (unreachable)
```

- Compact: one new glyph.
- ⚠ a11y caveat: loading vs unreachable are distinguished by **color
  alone** (same `○`) — fails colorblind-readable. Presented for
  completeness; weaker than B on the shape+color principle.

---

## Designer recommendation

**B** for operator-fidelity (loading ≠ unreachable, colorblind-safe),
or **A** if you prefer the minimal one-state fix and are fine with the
tooltip carrying the loading-vs-trouble detail. **Not C** — it
distinguishes by color alone.

Either way the alarm ceiling stays amber, never red.

## The other half — team-class, no design decision (#1636 (a))

`SyncStatusGlyph.test.tsx:39` only asserts the glyph EXISTS — the
vacuity that let false-green ship. Independent of this pick: pin each
of the 7 existing states (glyph + aria) AND the chosen new state(s).
Pure falsifier-strengthening; needs no owner-review.
