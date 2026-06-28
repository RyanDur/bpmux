import { describe, expect, test } from 'vitest'
import { stateDeltaDecoder, stateSnapshotDecoder } from './stateSyncDecoders'

describe('stateDeltaDecoder', () => {
  test('decodes a ci delta with null snapshot fields (serde Option -> null)', () => {
    const result = stateDeltaDecoder.validate({
      domain: 'ci',
      snapshot: { state: 'green', run_id: null, head_sha: null, display_title: null }
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok' && result.data.domain === 'ci') {
      expect(result.data.snapshot.state).toBe('green')
    }
  })

  test('decodes a roster delta (role/presence literals)', () => {
    const result = stateDeltaDecoder.validate({
      domain: 'roster',
      members: [{ routing_id: 'r', name: 'dev', identity: 'dev', role: 'Dev', presence: 'Busy' }]
    })
    expect(result.type).toBe('ok')
  })

  test('decodes board_invalidated and iteration_close (null deleted_marker_id)', () => {
    expect(stateDeltaDecoder.validate({ domain: 'board_invalidated', reason: 'x' }).type).toBe('ok')
    expect(
      stateDeltaDecoder.validate({
        domain: 'iteration_close',
        promoted: [1, 2],
        deleted_marker_id: null
      }).type
    ).toBe('ok')
  })

  test('decodes a ci_poll heartbeat and preserves last_successful_poll_at', () => {
    const result = stateDeltaDecoder.validate({
      domain: 'ci_poll',
      snapshot: { state: 'green', last_successful_poll_at: '2026-06-18T12:00:00Z' }
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok' && result.data.domain === 'ci_poll') {
      expect(result.data.snapshot.last_successful_poll_at).toBe('2026-06-18T12:00:00Z')
    }
  })

  test('decodes a sync_poll heartbeat carrying the full status', () => {
    const result = stateDeltaDecoder.validate({
      domain: 'sync_poll',
      status: {
        last_ingest_at: '2026-06-18T12:00:00Z',
        last_push_at: null,
        pending_events: 2,
        last_error: null,
        dropped_events: [],
        stale: true,
        startup_fail: false,
        conflicts_pending: 0
      }
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok' && result.data.domain === 'sync_poll') {
      expect(result.data.status.stale).toBe(true)
      expect(result.data.status.pending_events).toBe(2)
    }
  })

  test('decodes a roadmap delta of served flat cards carrying the enriched state', () => {
    const result = stateDeltaDecoder.validate({
      domain: 'roadmap',
      roadmap: [
        {
          number: 700,
          name: 'Now thing',
          why: { benefit: 'agents and humans share one view' },
          horizon: 'now',
          state: 'needs-attention',
          gap_count: 2,
          tone: 'gap'
        }
      ]
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok' && result.data.domain === 'roadmap') {
      expect(result.data.roadmap[0].state).toBe('needs-attention')
      expect(result.data.roadmap[0].gap_count).toBe(2)
      expect(result.data.roadmap[0].why.benefit).toBe('agents and humans share one view')
    }
  })

  test('tolerates a card whose why omits the absent fields (backend skips serializing None)', () => {
    const result = stateDeltaDecoder.validate({
      domain: 'roadmap',
      roadmap: [
        {
          number: 700,
          name: 'Lean why',
          why: {},
          horizon: 'later',
          state: 'planned',
          gap_count: 0,
          tone: 'future'
        }
      ]
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok' && result.data.domain === 'roadmap') {
      expect(result.data.roadmap[0].why.benefit).toBeUndefined()
      expect(result.data.roadmap[0].state).toBe('planned')
    }
  })

  test('rejects an unknown domain (story_transition is not GUI-consumed)', () => {
    expect(stateDeltaDecoder.validate({ domain: 'story_transition', transition: {} }).type).toBe(
      'error'
    )
  })

  test('rejects a bad ci state literal', () => {
    expect(stateDeltaDecoder.validate({ domain: 'ci', snapshot: { state: 'bogus' } }).type).toBe(
      'error'
    )
  })
})

describe('stateSnapshotDecoder', () => {
  test('decodes a snapshot with ci absent (skip_serializing_if)', () => {
    expect(stateSnapshotDecoder.validate({ reactions: [], roster: [] }).type).toBe('ok')
  })
})
