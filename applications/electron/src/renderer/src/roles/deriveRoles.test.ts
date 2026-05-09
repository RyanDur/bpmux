import { describe, expect, test } from 'vitest'
import {
  combinations,
  deriveCombinableRoles,
  deriveCombinableRolesFromRoster,
  deriveCombinatorialGroups,
  deriveDirectRoles,
  deriveDirectRolesFromRoster,
  deriveTeamRoles
} from './deriveRoles'
import type { ChatMessage } from '../transport/types'

const msg = (overrides: Partial<ChatMessage>): ChatMessage => ({
  id: 'm',
  from: '',
  text: '',
  ts: '2026-04-23T11:00:00Z',
  to: null,
  addressee: null,
  messageType: 'chat',
  ...overrides
})

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

describe('#393 deriveTeamRoles — runtime discovery from message stream (replaces desk-check-393.spec.ts)', () => {
  test('discovers all unique short-names across messages, sorted', () => {
    const messages = [
      msg({ from: 'ryan:electron' }),
      msg({ from: 'dev:claude-code' }),
      msg({ from: 'ops:claude-code' })
    ]
    expect(deriveTeamRoles(messages, 'ryan:electron')).toEqual(['dev', 'ops', 'ryan'])
  })

  test('includes a never-before-seen role (ops) outside the pre-#393 hardcoded list', () => {
    const messages = [msg({ from: 'ops:claude-code' })]
    expect(deriveTeamRoles(messages, 'ryan:electron')).toContain('ops')
  })

  test('omits designer/pm when neither speaks (no hardcoded fallback)', () => {
    const messages = [
      msg({ from: 'ryan:electron' }),
      msg({ from: 'dev:claude-code' })
    ]
    const roles = deriveTeamRoles(messages, 'ryan:electron')
    expect(roles).not.toContain('designer')
    expect(roles).not.toContain('pm')
  })

  test('viewer self always present even before having spoken', () => {
    expect(deriveTeamRoles([], 'ryan:electron')).toEqual(['ryan'])
  })

  test('to-recipients become roles even before speaking; "all" sentinel skipped', () => {
    const messages = [msg({ from: 'ryan:electron', to: ['dev', 'all'] })]
    const roles = deriveTeamRoles(messages, 'ryan:electron')
    expect(roles).toContain('dev')
    expect(roles).not.toContain('all')
  })
})

describe('#393 + #399 deriveDirectRoles — viewer + system senders excluded', () => {
  test('viewer (ryan) excluded from direct list', () => {
    const messages = [
      msg({ from: 'ryan:electron' }),
      msg({ from: 'dev:claude-code' })
    ]
    expect(deriveDirectRoles(messages, 'ryan:electron')).not.toContain('ryan')
  })

  test('bpmux excluded when it has emitted a system-typed message (#399)', () => {
    const messages = [
      msg({ from: 'ryan:electron' }),
      msg({ from: 'bpmux', messageType: 'system' }),
      msg({ from: 'dev:claude-code' })
    ]
    const directs = deriveDirectRoles(messages, 'ryan:electron')
    expect(directs).not.toContain('bpmux')
    expect(directs).toContain('dev')
  })

  test('a sender that only ever speaks chat-typed messages stays in the direct list', () => {
    const messages = [
      msg({ from: 'ryan:electron' }),
      msg({ from: 'dev:claude-code' })
    ]
    expect(deriveDirectRoles(messages, 'ryan:electron')).toContain('dev')
  })
})

describe('#386 deriveCombinableRoles + deriveCombinatorialGroups (replaces desk-check-386.spec.ts)', () => {
  test('three combinable roles yield exactly four groups (C(3,2)+C(3,3))', () => {
    const messages = [
      msg({ from: 'ryan:electron' }),
      msg({ from: 'bpmux', messageType: 'system' }),
      msg({ from: 'dev:claude-code' }),
      msg({ from: 'pm:claude-code' }),
      msg({ from: 'designer:claude-code' })
    ]
    const combinable = deriveCombinableRoles(messages, 'ryan:electron')
    expect(combinable).toEqual(['designer', 'dev', 'pm'])

    const groups = deriveCombinatorialGroups(combinable)
    expect(groups).toEqual([
      ['designer', 'dev'],
      ['designer', 'pm'],
      ['dev', 'pm'],
      ['designer', 'dev', 'pm']
    ])
  })

  test('owner (ryan) and system (bpmux) are absent from every group', () => {
    const messages = [
      msg({ from: 'ryan:electron' }),
      msg({ from: 'bpmux', messageType: 'system' }),
      msg({ from: 'dev:claude-code' }),
      msg({ from: 'pm:claude-code' }),
      msg({ from: 'designer:claude-code' })
    ]
    const combinable = deriveCombinableRoles(messages, 'ryan:electron')
    const groups = deriveCombinatorialGroups(combinable)
    for (const group of groups) {
      expect(group).not.toContain('ryan')
      expect(group).not.toContain('bpmux')
    }
  })

  test('fewer than two combinable roles produces no groups', () => {
    const messages = [
      msg({ from: 'ryan:electron' }),
      msg({ from: 'dev:claude-code' })
    ]
    const combinable = deriveCombinableRoles(messages, 'ryan:electron')
    expect(combinable).toEqual(['dev'])
    expect(deriveCombinatorialGroups(combinable)).toEqual([])
  })

  test('combinations enumerates only sizes within [min, max] inclusive', () => {
    const out = combinations(['a', 'b', 'c'], 2, 3)
    expect(out).toEqual([
      ['a', 'b'],
      ['a', 'c'],
      ['b', 'c'],
      ['a', 'b', 'c']
    ])
  })
})
