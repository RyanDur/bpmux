import { describe, expect, test } from 'vitest'
import { applyStateDelta, emptyMirrorState } from '../state-delta'
import type { MirrorState } from '../state-delta'
import type { StateDeltaPayload } from '../types'

const seededWith = (deltas: StateDeltaPayload[]): MirrorState =>
  deltas.reduce(applyStateDelta, emptyMirrorState())

describe('applyStateDelta', () => {
  test('merges reactions for a single channel without touching others', () => {
    const seeded = seededWith([
      {
        domain: 'reactions',
        channel: 'a',
        aggregated: [{ message_key: 'k1', emoji: '👍', count: 1, users: ['dev'] }]
      },
      {
        domain: 'reactions',
        channel: 'b',
        aggregated: [{ message_key: 'k2', emoji: '🎉', count: 1, users: ['owner'] }]
      }
    ])
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
    const seeded = seededWith([
      {
        domain: 'reactions',
        channel: 'a',
        aggregated: [{ message_key: 'k', emoji: '👍', count: 1, users: ['dev'] }]
      }
    ])
    const next = applyStateDelta(seeded, { domain: 'reactions', channel: 'a', aggregated: [] })
    expect(next.reactions.has('a')).toBe(false)
  })

  test('replaces ci snapshot in place', () => {
    const seeded = seededWith([{ domain: 'ci', snapshot: { state: 'green' } }])
    const next = applyStateDelta(seeded, {
      domain: 'ci',
      snapshot: { state: 'red', run_id: 9 }
    })
    expect(next.ci?.state).toBe('red')
    expect(next.ci?.run_id).toBe(9)
  })

  test('sync_poll heartbeat stores the full sync status in the mirror', () => {
    const next = applyStateDelta(emptyMirrorState(), {
      domain: 'sync_poll',
      status: {
        last_ingest_at: '2026-06-18T12:00:00Z',
        last_push_at: null,
        pending_events: 2,
        last_error: null,
        dropped_events: [],
        stale: false,
        startup_fail: false
      }
    })
    expect(next.sync?.pending_events).toBe(2)
    expect(next.sync?.last_ingest_at).toBe('2026-06-18T12:00:00Z')
  })

  test('ci_poll heartbeat refreshes the ci snapshot incl. last_successful_poll_at', () => {
    const seeded = seededWith([
      {
        domain: 'ci',
        snapshot: { state: 'green', last_successful_poll_at: '2026-01-01T00:00:00Z' }
      }
    ])
    const next = applyStateDelta(seeded, {
      domain: 'ci_poll',
      snapshot: { state: 'green', last_successful_poll_at: '2026-06-18T12:00:00Z' }
    })
    expect(next.ci?.state).toBe('green')
    expect(next.ci?.last_successful_poll_at).toBe('2026-06-18T12:00:00Z')
  })

  test('replaces roster wholesale', () => {
    const seeded = seededWith([
      {
        domain: 'roster',
        members: [
          {
            routing_id: 'sess-a',
            name: 'a',
            identity: 'a:claude-code',
            role: 'Dev',
            presence: 'Online'
          },
          {
            routing_id: 'sess-b',
            name: 'b',
            identity: 'b:claude-code',
            role: 'Pm',
            presence: 'Online'
          }
        ]
      }
    ])
    const next = applyStateDelta(seeded, {
      domain: 'roster',
      members: [
        {
          routing_id: 'sess-c',
          name: 'c',
          identity: 'c:claude-code',
          role: 'Designer',
          presence: 'Busy'
        }
      ]
    })
    expect(next.roster).toEqual([
      {
        routing_id: 'sess-c',
        name: 'c',
        identity: 'c:claude-code',
        role: 'Designer',
        presence: 'Busy'
      }
    ])
  })

  test('preserves prev reference identity for un-touched maps', () => {
    const seeded = seededWith([
      {
        domain: 'reactions',
        channel: 'a',
        aggregated: [{ message_key: 'k', emoji: '👍', count: 1, users: ['dev'] }]
      }
    ])
    const next = applyStateDelta(seeded, {
      domain: 'ci',
      snapshot: { state: 'green' }
    })
    expect(next.reactions).toBe(seeded.reactions)
  })

  test('returns prev (never undefined) for board_invalidated and any unknown future domain', () => {
    const seeded = seededWith([{ domain: 'ci', snapshot: { state: 'green' } }])
    const boardInvalidated = applyStateDelta(seeded, {
      domain: 'board_invalidated',
      reason: 'board changed'
    })
    expect(boardInvalidated).toBe(seeded)
    const unknownFuture = applyStateDelta(seeded, {
      domain: 'some_future_event'
    } as unknown as StateDeltaPayload)
    expect(unknownFuture).toBe(seeded)
    expect(unknownFuture.ci?.state).toBe('green')
  })
})
