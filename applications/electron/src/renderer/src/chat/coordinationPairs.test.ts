import { describe, expect, test } from 'vitest'
import { deriveCoordinationPairs } from './coordinationPairs'
import type { ChatMessage, RosterMemberWire } from '../transport/types'

const ROSTER: RosterMemberWire[] = [
  { name: 'dev', identity: 'dev:claude-code', role: 'Dev', presence: 'Online' },
  { name: 'pm', identity: 'pm:claude-code', role: 'Pm', presence: 'Online' },
  { name: 'designer', identity: 'designer:claude-code', role: 'Designer', presence: 'Online' },
  { name: 'ryan', identity: 'ryan:electron', role: 'Owner', presence: 'Online' }
]

let seq = 0
const msg = (over: Partial<ChatMessage>): ChatMessage => {
  seq += 1
  return {
    id: `m-${seq}`,
    from: 'dev:claude-code',
    text: 'coordinate',
    ts: `2026-06-03T00:00:${String(seq).padStart(2, '0')}.000Z`,
    to: null,
    addressee: null,
    messageType: null,
    ...over
  }
}

describe('#1605 deriveCoordinationPairs — addressee-derived who↔whom edges', () => {
  test('no messages yields no pairs', () => {
    expect(deriveCoordinationPairs([], ROSTER)).toEqual([])
  })

  test('a broadcast-with-addressee message IS a directed edge (the production-majority mode)', () => {
    const pairs = deriveCoordinationPairs(
      [msg({ from: 'dev:claude-code', addressee: 'pm', to: null })],
      ROSTER
    )
    expect(pairs).toHaveLength(1)
    expect(pairs[0]).toMatchObject({ agentA: 'dev', agentB: 'pm', count: 1 })
  })

  test('an un-addressed broadcast is NOT an edge (the "standup in 5" noise is excluded — compression)', () => {
    const pairs = deriveCoordinationPairs(
      [msg({ from: 'dev:claude-code', addressee: null, to: null })],
      ROSTER
    )
    expect(pairs).toEqual([])
  })

  test('a to-DM (no addressee) is a directed edge', () => {
    const pairs = deriveCoordinationPairs(
      [msg({ from: 'dev:claude-code', addressee: null, to: ['pm'] })],
      ROSTER
    )
    expect(pairs).toHaveLength(1)
    expect(pairs[0]).toMatchObject({ agentA: 'dev', agentB: 'pm' })
  })

  test('addressee takes precedence over the to: privacy-filter for the coordination target', () => {
    const pairs = deriveCoordinationPairs(
      [msg({ from: 'dev:claude-code', addressee: 'pm', to: ['designer'] })],
      ROSTER
    )
    expect(pairs).toHaveLength(1)
    expect(pairs[0]).toMatchObject({ agentA: 'dev', agentB: 'pm' })
  })

  test('A→B and B→A aggregate into ONE canonical bidirectional pair (↔), count summed, lastTs latest', () => {
    const pairs = deriveCoordinationPairs(
      [
        msg({ from: 'dev:claude-code', addressee: 'pm', ts: '2026-06-03T00:00:01.000Z' }),
        msg({ from: 'pm:claude-code', addressee: 'dev', ts: '2026-06-03T00:00:09.000Z' })
      ],
      ROSTER
    )
    expect(pairs).toHaveLength(1)
    expect(pairs[0]).toMatchObject({
      agentA: 'dev',
      agentB: 'pm',
      count: 2,
      lastTs: '2026-06-03T00:00:09.000Z'
    })
  })

  test('N≥2 distinct coordinating pairs render as DISTINCT entries — never collapsed to one feed (non-vacuous guard)', () => {
    const pairs = deriveCoordinationPairs(
      [
        msg({ from: 'dev:claude-code', addressee: 'pm' }),
        msg({ from: 'designer:claude-code', addressee: 'dev' })
      ],
      ROSTER
    )
    expect(pairs).toHaveLength(2)
    const keys = pairs.map((p) => `${p.agentA}-${p.agentB}`).sort()
    expect(keys).toEqual(['designer-dev', 'dev-pm'])
  })

  test('the owner is excluded as a coordination edge (constant observer, not a participant edge)', () => {
    const pairs = deriveCoordinationPairs(
      [
        msg({ from: 'dev:claude-code', addressee: 'ryan' }),
        msg({ from: 'ryan:electron', addressee: 'dev' })
      ],
      ROSTER
    )
    expect(pairs).toEqual([])
  })

  test('the system sender (bpmux) is excluded', () => {
    const pairs = deriveCoordinationPairs([msg({ from: 'bpmux', addressee: 'dev' })], ROSTER)
    expect(pairs).toEqual([])
  })

  test('a broadcast-attention addressee ("all") is not a pair', () => {
    const pairs = deriveCoordinationPairs(
      [msg({ from: 'dev:claude-code', addressee: 'all' })],
      ROSTER
    )
    expect(pairs).toEqual([])
  })

  test('a self-addressed message is not a pair', () => {
    const pairs = deriveCoordinationPairs(
      [msg({ from: 'dev:claude-code', addressee: 'dev' })],
      ROSTER
    )
    expect(pairs).toEqual([])
  })

  test('pairs are ordered most-RECENT-first — recency is the active-now bird’s-eye signal, not volume', () => {
    const pairs = deriveCoordinationPairs(
      [
        msg({ from: 'dev:claude-code', addressee: 'pm', ts: '2026-06-03T00:00:01.000Z' }),
        msg({ from: 'pm:claude-code', addressee: 'dev', ts: '2026-06-03T00:00:02.000Z' }),
        msg({ from: 'designer:claude-code', addressee: 'dev', ts: '2026-06-03T00:00:09.000Z' })
      ],
      ROSTER
    )
    expect(pairs.map((p) => `${p.agentA}-${p.agentB}`)).toEqual(['designer-dev', 'dev-pm'])
    expect(pairs[0]).toMatchObject({
      agentA: 'designer',
      agentB: 'dev',
      count: 1,
      lastTs: '2026-06-03T00:00:09.000Z'
    })
  })
})
