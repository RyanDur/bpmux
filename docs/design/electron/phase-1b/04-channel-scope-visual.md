# phase-1b channel scope visual treatment

Ryan 2026-04-22 ~22:48 UTC during acceptance walk: *"I just checked
the terminal version and all the channels are hidden from me. I am in
broadcast and I can't see the channels. this is against the idea of
bpmux. nothing is supposed to be hidden from me. I can filter my view
but I should be able to control my scope not have it hidden from me."*

Channels currently act as hide-walls. They should act as
emphasis-shifts. #385 establishes the semantic; this spec names the
visual vocabulary.

## 3-field spec header

**user-job:** When Ryan (or any agent) selects a channel (broadcast,
DM, group, system), they need the stream to emphasize that channel's
messages while keeping all other messages fully visible, because
owner-scope is a focus refinement, not a visibility wall. Hiding
violates the collaboration-visibility-inviolable principle.

**observation-falsifier:** I would reject if, while any channel is
selected, any message (broadcast / DM I'm part of / group I'm part of
/ system / any message type) disappears from the chat stream, OR if
any message becomes illegible due to dim. The only way to hide
messages is an explicit opt-in class filter pill toggle (the existing
#349 substrate).

**team-shape-independence:** channel scope derives from message
`to` field + viewer identity (from `get_identity`, not hardcoded).
Works for any agent composition.

## principle alignments

- **nothing-hidden-from-owner** (2026-04-22 principle): all messages
  always render in the stream. Dim ≠ hide; dim reduces emphasis but
  preserves readability.
- **collaboration-visibility-inviolable**: chat input always reachable
  (unchanged), chat stream always complete (new guarantee).
- **unified secondary-emphasis vocabulary**: same opacity tier as
  addressed-to-other treatment (phase-1b/01). One "secondary" visual
  language, not two.

## visual rules

Precedence (highest-emphasis first):

1. **Primary (opacity 1.0):** message meets EITHER condition:
   - addressed-to-viewer (`to` includes viewer's role)
   - broadcast AND current-channel is "all" or "broadcast"
   - matches current-channel selection (msg's channel == selected
     channel)

2. **Secondary (opacity 0.82):** message does NOT meet any primary
   condition. Secondary = "present, not focused" — fully readable,
   clearly deemphasized.

**No double-dim stacking.** A message that's both to-other AND
not-in-current-channel is still 0.82, not 0.67. Either condition →
0.82 floor, not compounded.

**0.82 is the established floor** from phase-1b/01-chat-readability
addressed-to-other treatment. WCAG AA contrast verified on that
opacity against the chat background with `--fg-primary`. Same token,
same tier. Do not invent a new opacity level for channels.

## current-channel indicator

The selected channel in the sidebar already renders with existing
selected-state styling (brand-color accent / filled indicator). No
change required — emphasis in the STREAM follows from the sidebar
state, not from a second selection indicator in the header.

Header may optionally add a small scope label ("viewing: #design" or
"viewing: DM with pm") for explicitness, but this is optional polish
— the sidebar state is load-bearing signal.

## input scope

Input scope follows channel selection (existing behavior, preserved):
- broadcast channel selected → input sends broadcast
- DM channel selected → input sends DM to that role
- group channel selected → input sends to that group
- system (if channel exists) → input disabled or restricted

Input affordance communicates scope (placeholder text, border color,
or pill). This spec does NOT redesign input scope; it preserves current
behavior.

## stream rendering

No DOM restructure. Messages render in chronological order as today.
Opacity is the ONLY visual change per message — applied via a CSS
class toggle:

```
.chat-message                      /* default: opacity 1 */
.chat-message[data-scope="dim"]    /* opacity 0.82 */
```

The `data-scope` attribute gets set by the renderer's filter logic
based on: viewer's role (from `get_identity` where available), current
channel selection, message's `to` field. Attribute-driven styling is
consistent with `data-theme` / `data-density` / `data-lanes` patterns
already in the CSS substrate.

## empty-state behavior

If current channel has no matching messages: stream is NOT empty.
Stream shows all other messages dimmed. Input scope communicates
"you're viewing channel X" via placeholder. No "nothing to see"
splash, no "select a different channel" prompt.

Why: nothing-hidden means the stream is always the complete stream.
Channel-scope affects emphasis, not presence. Empty-state UX would
suggest the stream IS empty (false — other channels are there,
dimmed).

## filter-pill coexistence

The existing #349 class-filter pills (chat / insight / activity /
system) remain the only mechanism for OPT-IN hiding. Those filter
by message class, not by channel. Channel-scope is orthogonal:

| control | action | hide semantic |
|---------|--------|---------------|
| channel select | shifts emphasis | no — always visible |
| class filter pill (#349) | toggles visibility | yes — explicit opt-in |

Both can be active simultaneously. Class filter hides THEN
channel-scope dims remaining. Rendering precedence: hide first,
then apply opacity to what's visible.

## TUI parity

TUI has no opacity primitive in the `tui` terminal-cell substrate, so
the equivalent treatment uses:
- **current-channel messages:** default color
- **non-current-channel messages:** `fg-secondary` color (dim-equivalent
  via color, not opacity)

Same principle, TUI-appropriate affordance. Dev's lane to implement —
this spec names the principle; TUI implementation details are dev-
lane.

## acceptance criteria (augmenting #385 filed ACs)

1. Any message in the stream is readable (visible + legible) regardless
   of current channel selection.
2. Current-channel messages + addressed-to-viewer messages render at
   full opacity (primary).
3. Other-channel, other-addressee messages render at 0.82 opacity
   (secondary). No compounding when both conditions apply.
4. Switching channels does NOT remove messages from the stream — only
   changes opacity classes.
5. Empty-current-channel state: stream is NOT empty; shows all messages
   with current-channel matches empty but other messages still visible
   (dimmed).
6. Class filter pills (#349) still hide by class; combine with channel
   scope without conflict.
7. WCAG AA contrast preserved at 0.82 opacity (inherited from
   phase-1b/01).
8. `data-scope="dim"` attribute correctly set on non-primary messages;
   verifiable via Playwright DOM assertion.
9. Playwright real-backend test: seed broadcast + DM + group + system
   messages; switch channels; assert every message is in DOM + visible
   across all channel selections (by `data-test` selector), opacity
   class differs per scope.
10. TUI equivalent: non-current-channel messages render in
    `fg-secondary` color; all messages remain in the chat panel.

## desk-check walk script (designer user-story walk)

Fixture: 4 messages — (a) broadcast, (b) DM to viewer, (c) DM between
two other roles, (d) group message viewer not in.

Walk as Ryan:
1. In broadcast channel, can I see all 4? (a) + (b) primary; (c) + (d)
   dim but visible.
2. Switch to DM-with-pm channel. Can I still see all 4? (b) if from
   pm = primary; all others dim (including (a) broadcast).
3. Verify at 0.82 opacity each dim message is still fully legible —
   not a fade-to-unreadable.
4. Class filter: toggle "hide activity" pill. Activity messages hide
   across all channels; non-activity messages still render per scope
   rules.
5. Empty case: switch to a channel with no messages matching the
   viewer's scope. Stream still shows dim messages from other
   channels. Input placeholder communicates "message #channel".
6. Same test in TUI: `fg-secondary` color for non-current-channel
   messages.

Walk pass = observation-falsifier doesn't fire at any step + no message
disappears from DOM at any channel switch.

## out of scope

- **#332 sidebar DM picker emphasis treatment** — separate story;
  builds on this spec. When Ryan clicks "designer" in the sidebar, it
  sets designer-DM as the current channel; emphasis follows these
  rules. Visual affordance for the sidebar-click → channel-set
  transition is #332's lane.
- **Channel-scope as persistent URL state** — if Ryan wants "remember
  last channel on reload," that's a separate feature.
- **Per-role custom emphasis** — uniform 0.82 for now; no
  individualized tuning.
- **Collapsible "3 DM messages" summary rows** — rejected at spec
  time. Default-collapsed = hidden-in-spirit, violates
  nothing-hidden principle.
