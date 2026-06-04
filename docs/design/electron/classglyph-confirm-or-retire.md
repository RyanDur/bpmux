# ClassGlyph — confirm-or-retire analysis (#3 owner-bundle item)

Designer right-thing analysis seeding a confirm-or-retire story. PM/dev
files the issue (designer can't `create_story`); this is the authored
substance.

## What ClassGlyph is (grounded)

A faint 12×12 SVG rendered before each NON-chat message, distinguishing
two automated message-classes:
- **EventsGlyph** — concentric circles (`ClassGlyph.tsx:8-23`)
- **BpmuxGlyph** — 8-point star/gear (`ClassGlyph.tsx:25-45`)
- chat-class messages get NO glyph (`ClassGlyph.tsx:51-55` returns null)

Discriminator: `cls={classOf(m.messageType)}` — `classOf` +
`LEGACY_TYPE_REMAP` in `useMessageFilter.ts:41-56` map `system`→bpmux;
`insight`/`activity`/`state-delta`/`state-snapshot`/`nudge`→events;
else→chat. Color `--fg-dim`, `role="img"` + aria-label.

**Single render site (verified):** `MessageItem.tsx:7` (import) + `:123`
(render). The phase-1b reuse-references
(`03-blocker-visual.md:71`, `07-lane-separation-reshape.md`) are
**aspirational design-notes, not live usage**.

## Why confirm-or-retire, NOT polish

Ryan, on seeing it: **"I don't know what that is."** Per the
right-thing-audit, **owner non-recognition is the strongest evidence of
un-grounded build-ahead** — a marker the owner never asked for and
doesn't recognize. So the **default disposition is RETIRE** the
two-SVG distinction. We must NOT half-build a *simpler* version of an
un-raised marker — that compounds the un-grounded build.

## Pickup gate — do this FIRST (before any code change)

Confirm with Ryan: **do you want an automated-vs-human (events-vs-bpmux)
per-message marker AT ALL?**
- **NO (expected default) → retire** (scope below).
- **YES → reshape** into a proper spec (what it should distinguish +
  why, owner-reviewed). Do NOT restyle the current two-SVG blind.

## Retire scope (verified firsthand — bounded)

| remove | keep |
|---|---|
| `ClassGlyph.tsx` (whole component) | `classOf` / `LEGACY_TYPE_REMAP` in `useMessageFilter.ts` — **used by the message-filter UI independently; do NOT delete** |
| `MessageItem.tsx:7` import + `:123` render | chat-class message rendering (unchanged) |
| `.message-class-glyph` CSS (`MessageList.css:442+`) | |
| `__tests__/human-message-class-glyphs-render.test.tsx` (glyph-specific) | |
| glyph-assertions WITHIN `desk-check-373.spec.ts` (72-115), `desk-check-349.spec.ts`, `real-backend.spec.ts` — assertions only, NOT whole specs (they cover other jobs) | |
| `LINT_BASELINE.md:44` entry | |
| annotate/strip the phase-1b reuse-references | |

## Filing payload (for PM/dev `create_story`)

- **title:** `[Chore] ClassGlyph confirm-or-retire — per-message events/bpmux automated-marker is owner-unrecognized; default retire unless owner confirms it's wanted`
- **user_job:** When Ryan scans the chat, he needs every persistent
  visual marker to correspond to a need he recognizes — the per-message
  events/bpmux glyph is owner-unrecognized ("I don't know what that
  is"), the strongest signal it's un-grounded build-ahead to retire
  unless a need surfaces.
- **observation_falsifier:** I would reject if the glyph is simplified/
  restyled in place WITHOUT first confirming with the owner that an
  automated-vs-human marker is wanted at all (half-building an un-raised
  thing); OR if a retire leaves dead residue (ClassGlyph, the CSS, the
  glyph tests, the LINT_BASELINE entry) OR wrongly deletes `classOf`
  (used by the filter UI); OR if chat-class rendering changes; OR if the
  phase-1b reuse-references are left dangling at a removed component.
- **filing-class:** Chore (cleanup; adds no capability). Owner-confirm
  gate at pickup; if retire, dev-accepts once dead-code-free + green.
- **priority:** low (rides Grus as shaping-work, promote-when-shaped).
