import type { ChatMessage } from '../transport/types'
import type { RosterEntry } from '../hooks/useTeamRoster'

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
 * viewer's perspective — every role except the viewer's own AND
 * system senders. Sidebar's "direct" section renders these.
 *
 * #399 + Ryan 2026-04-23 11:51 UTC: "bpmux is not something we
 * have conversations with. it is the medium in which we
 * communicate." System-typed senders are the transport layer;
 * treating them as DM targets invites users to address the
 * channel-substrate itself. Detection is structural (via
 * `messageType === 'system'`) — no literal name strings —
 * mirroring the same category filter #386's
 * `deriveCombinableRoles` uses for group enumeration.
 */
export function deriveDirectRoles(
  messages: readonly ChatMessage[],
  viewer?: string
): readonly string[] {
  const all = deriveTeamRoles(messages, viewer)
  const systemSenders = deriveSystemSenders(messages)
  const self = viewer !== undefined ? shortName(viewer) : null
  return all.filter((r) => r !== self && !systemSenders.has(r))
}

/**
 * Enumerate all combinatorial subsets of `roles` with size in
 * `[minSize, maxSize]` inclusive. Output is sorted canonically
 * — each subset's role list is alphabetical, and the outer list
 * is ordered by size ascending then lexicographic. Pure function;
 * stable across re-renders given the same input.
 */
export function combinations(
  roles: readonly string[],
  minSize: number,
  maxSize: number
): string[][] {
  const sorted = [...roles].sort()
  const out: string[][] = []
  const n = sorted.length
  const clampedMin = Math.max(1, minSize)
  const clampedMax = Math.min(n, maxSize)
  if (clampedMin > clampedMax) return out
  for (let size = clampedMin; size <= clampedMax; size++) {
    const acc: string[] = []
    const walk = (start: number): void => {
      if (acc.length === size) {
        out.push([...acc])
        return
      }
      const remaining = size - acc.length
      for (let i = start; i <= n - remaining; i++) {
        acc.push(sorted[i])
        walk(i + 1)
        acc.pop()
      }
    }
    walk(0)
  }
  return out
}

/**
 * #386 + Ryan 2026-04-23 11:47 UTC: the owner (viewer) and any
 * system senders are architectural CONSTANTS — always present in
 * every conversation — not combinatorial variables. They vary
 * over zero bits. Sidebar group enumeration must skip them.
 *
 * Identification is structural, not by hardcoded name:
 *   - viewer: caller-provided (already pluggable per #393 AC1).
 *     The renderer's current identity, whoever that is.
 *   - system senders: any `from` that has emitted at least one
 *     message with `messageType === 'system'`. In bpmux this is
 *     the `bpmux` bridge sender; the detector scales to any
 *     future system-typed emitter without a name-literal.
 */
function deriveSystemSenders(
  messages: readonly ChatMessage[]
): ReadonlySet<string> {
  const set = new Set<string>()
  for (const m of messages) {
    if (m.messageType === 'system' && m.from) {
      set.add(shortName(m.from))
    }
  }
  return set
}

/**
 * The "combinable" role set — observed team minus the viewer
 * (always-present owner) and system senders (always-present
 * bridge). This is the alphabet the sidebar's group enumeration
 * iterates over.
 */
export function deriveCombinableRoles(
  messages: readonly ChatMessage[],
  viewer?: string
): readonly string[] {
  const all = deriveTeamRoles(messages, viewer)
  const systemSenders = deriveSystemSenders(messages)
  const self = viewer !== undefined ? shortName(viewer) : null
  return all.filter((r) => r !== self && !systemSenders.has(r))
}

/**
 * Convenience for the #386 sidebar case: enumerate all size ≥ 2
 * subsets of the combinable-role set (team-minus-constants).
 * Single-role subsets are the Direct-DM section; this is the
 * multi-role Groups section.
 *
 * 3 combinable roles → C(3,2) + C(3,3) = 4 subsets.
 * 4 combinable roles → C(4,2) + C(4,3) + C(4,4) = 11 subsets.
 */
export function deriveCombinatorialGroups(
  roles: readonly string[]
): string[][] {
  if (roles.length < 2) return []
  return combinations(roles, 2, roles.length)
}

// ── #402 roster-based derivations ────────────────────────────────
//
// Membership authority switches from observed message senders to the
// connected-agent roster (fetched via `/api/roster`, refreshed on
// `ROSTER_UPDATE_MARKER`). The helpers below mirror the message-
// based ones in signature but read from the roster snapshot instead.
// The SAME filter rules apply — viewer exclusion + system-sender
// exclusion — only the input axis changes. This preserves #393 AC1
// team-shape-independence + #386's category detection + #399's
// no-DM-to-the-medium semantic.

/**
 * The full roster-derived team role set. Includes the viewer if
 * present in the roster (which it typically is, since Electron's
 * renderer holds a WS connection). Sorted for stable iteration.
 */
export function deriveTeamRolesFromRoster(
  roster: readonly RosterEntry[]
): readonly string[] {
  const set = new Set<string>()
  for (const entry of roster) set.add(entry.role)
  return Array.from(set).sort()
}

/**
 * Direct-DM candidates = roster minus viewer. No message-history
 * dependence, so a teammate stays visible across hydration-window
 * changes as long as their WS session stays up. System senders
 * (bpmux) aren't in the roster at all — bpmux emits system
 * messages but never holds a WS connection — so no isSystem filter
 * is needed at this layer.
 */
export function deriveDirectRolesFromRoster(
  roster: readonly RosterEntry[],
  viewer?: string
): readonly string[] {
  const all = deriveTeamRolesFromRoster(roster)
  const self = viewer !== undefined ? shortName(viewer) : null
  return all.filter((r) => r !== self)
}

/**
 * Combinable set for the groups-enumeration path. Identical to the
 * directs set today (roster excludes bpmux; viewer is filtered).
 * Kept as a distinct export so future divergence (e.g., a role
 * that's DM-able but not group-addressable) has a clean extension
 * point.
 */
export function deriveCombinableRolesFromRoster(
  roster: readonly RosterEntry[],
  viewer?: string
): readonly string[] {
  return deriveDirectRolesFromRoster(roster, viewer)
}
