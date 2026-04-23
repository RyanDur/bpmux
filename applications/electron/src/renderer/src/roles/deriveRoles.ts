import type { ChatMessage } from '../transport/types'

// #393 team-shape audit (AC1).
//
// Before: DIRECT_ROLES / KNOWN_ROLES / PRESENCE_ROLES were three
// hardcoded constants at three different files, each naming the
// current bpmux team (ryan / dev / pm / designer). Adding a 5th
// role required code edits in ≥3 files; any agent outside the
// hardcoded list rendered as "unknown" with no avatar color.
//
// After: role set is derived at runtime from the observed message
// stream. Any agent that has spoken in the session (or the viewer
// itself) is a first-class role — sidebar entry, presence tracking,
// data-role attribute, avatar styling. No hardcoded team-composition
// in the derivation layer.
//
// The CSS palette still ships with --role-ryan / --role-dev / etc
// tokens; unknown roles fall through to the existing `--role-unknown`
// token (neutral color). Dynamic palette generation is AC2 / a
// follow-up chore.

/**
 * Strip any ':transport' suffix from an identity string so `dev:claude-code`
 * and `dev:cursor` both resolve to the same short-role key. Mirrors
 * the `shortName` helper in MessageList; duplicated here to keep this
 * module standalone.
 */
function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

/**
 * Derive the full set of role short-names present in the current
 * session. Sources:
 *   - unique `from` short-names across all observed messages
 *   - optional `viewer` (the renderer's own identity) so the
 *     current session always appears as a role even before it
 *     has spoken
 *   - optional `to` recipients — a role addressed via DM/group
 *     but not yet speaking still counts as a known team member
 *
 * Returns a sorted array for stable iteration. Empty message log
 * + no viewer + no addressees → empty array; callers that need a
 * fallback render "unknown" when a sender isn't in the set, same
 * as pre-#393.
 */
export function deriveTeamRoles(
  messages: readonly ChatMessage[],
  viewer?: string
): readonly string[] {
  const set = new Set<string>()
  if (viewer !== undefined && viewer.length > 0) set.add(shortName(viewer))
  for (const m of messages) {
    if (m.from && m.from.length > 0) set.add(shortName(m.from))
    if (m.to) {
      for (const t of m.to) {
        // Exclude the broadcast sentinel; it's an address, not a
        // role.
        if (t !== 'all' && t.length > 0) set.add(t)
      }
    }
  }
  return Array.from(set).sort()
}

/**
 * Subset of derived roles that are "direct-DM-able" from the
 * viewer's perspective — every role except the viewer's own.
 * Sidebar's "direct" section renders these; messaging yourself
 * isn't a meaningful DM target.
 */
export function deriveDirectRoles(
  messages: readonly ChatMessage[],
  viewer?: string
): readonly string[] {
  const all = deriveTeamRoles(messages, viewer)
  if (viewer === undefined) return all
  const self = shortName(viewer)
  return all.filter((r) => r !== self)
}
