import { describe, expect, test } from 'vitest'
import { deriveCombinableRolesFromRoster, deriveDirectRolesFromRoster } from './deriveRoles'

describe('#495 deriveDirectRolesFromRoster — orphan agent-XXXX dedup', () => {
  test('filters orphan agent when same-tool named entry exists', () => {
    const roster = [
      { identity: 'agent-abcd:claude-code', role: 'agent-abcd' },
      { identity: 'dev:claude-code', role: 'dev' }
    ]
    expect(deriveDirectRolesFromRoster(roster, 'ryan:human')).toEqual(['dev'])
  })

  test('preserves truly-distinct unnamed agents on different tools', () => {
    const roster = [
      { identity: 'agent-abcd:claude-code', role: 'agent-abcd' },
      { identity: 'agent-xyzw:cursor', role: 'agent-xyzw' }
    ]
    expect(deriveDirectRolesFromRoster(roster, 'ryan:human')).toEqual([
      'agent-abcd',
      'agent-xyzw'
    ])
  })

  test('filters viewer self from output', () => {
    const roster = [
      { identity: 'ryan:human', role: 'ryan' },
      { identity: 'dev:claude-code', role: 'dev' }
    ]
    expect(deriveDirectRolesFromRoster(roster, 'ryan:human')).toEqual(['dev'])
  })

  test('handles empty roster', () => {
    expect(deriveDirectRolesFromRoster([], 'ryan:human')).toEqual([])
  })

  test('groups derive uses same dedup', () => {
    const roster = [
      { identity: 'agent-abcd:claude-code', role: 'agent-abcd' },
      { identity: 'dev:claude-code', role: 'dev' },
      { identity: 'pm:claude-code', role: 'pm' }
    ]
    expect(deriveCombinableRolesFromRoster(roster, 'ryan:human')).toEqual(['dev', 'pm'])
  })
})
