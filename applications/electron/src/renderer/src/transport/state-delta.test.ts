import { describe, expect, test } from 'vitest'
import { applyStateDelta, applyStateSnapshot, emptyMirrorState } from './state-delta'
import type { StateDeltaPayload, StateSnapshotPayload } from './types'

describe('applyStateSnapshot', () => {
  test('replaces all domains from a fresh snapshot', () => {
    const snapshot: StateSnapshotPayload = {
      reactions: [
        {
          channel: 'broadcast',
          aggregated: [{ message_key: 'k1', emoji: '👍', count: 2, users: ['ryan', 'dev'] }]
        }
      ],
      ci: { state: 'green', run_id: 7, head_sha: 'abc', display_title: 'title' },
      roster: [{ name: 'ryan', identity: 'ryan:electron', role: 'Owner', presence: 'Online' }]
    }
    const next = applyStateSnapshot(emptyMirrorState(), snapshot)
    expect(next.reactions.get('broadcast')?.[0].count).toBe(2)
    expect(next.ci?.state).toBe('green')
    expect(next.roster[0].name).toBe('ryan')
  })

  test('clears domains when snapshot omits them', () => {
    const seeded = applyStateSnapshot(emptyMirrorState(), {
      reactions: [
        {
          channel: 'old',
          aggregated: [{ message_key: 'k', emoji: '🎉', count: 1, users: ['dev'] }]
        }
      ],
      ci: { state: 'red' },
      roster: [{ name: 'dev', identity: 'dev:claude-code', role: 'Dev', presence: 'Online' }]
    })
    const cleared = applyStateSnapshot(seeded, {
      reactions: [],
      ci: null,
      roster: []
    })
    expect(cleared.reactions.size).toBe(0)
    expect(cleared.ci).toBeNull()
    expect(cleared.roster).toEqual([])
  })
})

describe('applyStateDelta', () => {
  test('merges reactions for a single channel without touching others', () => {
    const seeded = applyStateSnapshot(emptyMirrorState(), {
      reactions: [
        {
          channel: 'a',
          aggregated: [{ message_key: 'k1', emoji: '👍', count: 1, users: ['dev'] }]
        },
        {
          channel: 'b',
          aggregated: [{ message_key: 'k2', emoji: '🎉', count: 1, users: ['ryan'] }]
        }
      ],
      ci: null,
      roster: []
    })
    const delta: StateDeltaPayload = {
      domain: 'reactions',
      channel: 'a',
      aggregated: [{ message_key: 'k1', emoji: '👍', count: 2, users: ['dev', 'pm'] }]
    }
    const next = applyStateDelta(seeded, delta)
    expect(next.reactions.get('a')?.[0].count).toBe(2)
    expect(next.reactions.get('b')?.[0].count).toBe(1)
  })

  test('removes empty reactions channel from the map', () => {
    const seeded = applyStateSnapshot(emptyMirrorState(), {
      reactions: [
        { channel: 'a', aggregated: [{ message_key: 'k', emoji: '👍', count: 1, users: ['dev'] }] }
      ],
      ci: null,
      roster: []
    })
    const next = applyStateDelta(seeded, { domain: 'reactions', channel: 'a', aggregated: [] })
    expect(next.reactions.has('a')).toBe(false)
  })

  test('replaces ci snapshot in place', () => {
    const seeded = applyStateSnapshot(emptyMirrorState(), {
      reactions: [],
      ci: { state: 'green' },
      roster: []
    })
    const next = applyStateDelta(seeded, {
      domain: 'ci',
      snapshot: { state: 'red', run_id: 9 }
    })
    expect(next.ci?.state).toBe('red')
    expect(next.ci?.run_id).toBe(9)
  })

  test('replaces roster wholesale', () => {
    const seeded = applyStateSnapshot(emptyMirrorState(), {
      reactions: [],
      ci: null,
      roster: [
        { name: 'a', identity: 'a:claude-code', role: 'Dev', presence: 'Online' },
        { name: 'b', identity: 'b:claude-code', role: 'Pm', presence: 'Online' }
      ]
    })
    const next = applyStateDelta(seeded, {
      domain: 'roster',
      members: [{ name: 'c', identity: 'c:claude-code', role: 'Designer', presence: 'Busy' }]
    })
    expect(next.roster).toEqual([
      { name: 'c', identity: 'c:claude-code', role: 'Designer', presence: 'Busy' }
    ])
  })

  test('preserves prev reference identity for un-touched maps', () => {
    const seeded = applyStateSnapshot(emptyMirrorState(), {
      reactions: [
        { channel: 'a', aggregated: [{ message_key: 'k', emoji: '👍', count: 1, users: ['dev'] }] }
      ],
      ci: null,
      roster: []
    })
    const next = applyStateDelta(seeded, {
      domain: 'ci',
      snapshot: { state: 'green' }
    })
    expect(next.reactions).toBe(seeded.reactions)
  })
})
