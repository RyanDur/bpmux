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
