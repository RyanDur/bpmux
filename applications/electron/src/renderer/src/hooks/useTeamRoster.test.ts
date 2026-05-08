import { describe, expect, test } from 'vitest'
import { selectRoster, wireToRosterEntry, type RosterEntry } from './useTeamRoster'
import type { RosterMemberWire } from '../transport/types'

const wire = (overrides: Partial<RosterMemberWire> = {}): RosterMemberWire => ({
  name: 'dev',
  identity: 'dev:claude-code',
  role: 'Dev',
  presence: 'Online',
  ...overrides
})

describe('selectRoster — slice 5g roster mirror-priority archetype', () => {
  test('returns mapped mirror entries when mirror has data (mirror-priority over REST)', () => {
    const mirror: RosterMemberWire[] = [wire(), wire({ name: 'pm', identity: 'pm:claude-code' })]
    const rest: RosterEntry[] = [{ identity: 'stale:rest', role: 'stale' }]
    const result = selectRoster(mirror, rest)
    expect(result.length).toBe(2)
    expect(result[0].identity).toBe('dev:claude-code')
    expect(result[0].role).toBe('dev')
    expect(result[1].identity).toBe('pm:claude-code')
  })

  test('falls back to REST roster when mirror is empty', () => {
    const rest: RosterEntry[] = [{ identity: 'designer:claude-code', role: 'designer' }]
    expect(selectRoster([], rest)).toBe(rest)
    expect(selectRoster(undefined, rest)).toBe(rest)
  })
})

describe('wireToRosterEntry — slice 5g vocabulary translation', () => {
  test('maps RosterMemberWire.identity → RosterEntry.identity + name → role', () => {
    const entry = wireToRosterEntry(wire({ name: 'designer', identity: 'designer:claude-code' }))
    expect(entry.identity).toBe('designer:claude-code')
    expect(entry.role).toBe('designer')
  })

  test('preserves transport-suffix needed by deriveDirectRolesFromRoster.toolSuffixOf', () => {
    const entry = wireToRosterEntry(wire({ name: 'agent-abc', identity: 'agent-abc:claude-code' }))
    expect(entry.identity).toBe('agent-abc:claude-code')
    expect(entry.identity.split(':')[1]).toBe('claude-code')
  })
})
