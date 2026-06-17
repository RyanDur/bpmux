import { describe, expect, test } from 'vitest'
import { boardResponseDecoder } from './boardDecoder'

const validBoard = {
  fetched_at: '2026-06-17T00:00:00Z',
  stories: [
    {
      number: 1,
      title: 'minimal story',
      state: 'ready',
      labels: ['Feature'],
      assignees: ['dev'],
      created_at: '2026-06-17T00:00:00Z',
      body: ''
    },
    {
      number: 2,
      title: 'fully-populated story',
      state: 'in-progress',
      gh_state: 'open',
      labels: ['iteration'],
      assignees: [],
      created_at: '2026-06-17T00:00:00Z',
      body: 'b',
      parent_epic: 100,
      iteration_membership: { iteration_number: 10, position: 1 },
      iteration_order: 2,
      is_current: true,
      acceptor: 'owner',
      rework_comment: null,
      horizon: 'now',
      outcome_readiness: { iteration_count: 2, gap_count: 1, all_ready: false },
      outcome_membership: { outcome_number: 5, position: 1 },
      readiness: { drafted: true, stories_ordered: true, review_closed: false, dev_ok: false }
    }
  ]
}

describe('boardResponseDecoder', () => {
  test('decodes a valid response (minimal + fully-populated stories, nested readiness/membership)', () => {
    const result = boardResponseDecoder.validate(validBoard)
    expect(result.type).toBe('ok')
    if (result.type === 'ok') {
      expect(result.data.stories).toHaveLength(2)
      expect(result.data.stories[1].readiness?.review_closed).toBe(false)
      expect(result.data.stories[1].iteration_membership?.iteration_number).toBe(10)
      expect(result.data.stories[1].rework_comment).toBeNull()
    }
  })

  test('decodes parent_epic: null (serde Option::None) — the real-backend shape the old cast silently mis-typed', () => {
    const board = {
      fetched_at: 'x',
      stories: [
        {
          number: 1,
          title: 't',
          state: 'ready',
          labels: [],
          assignees: [],
          created_at: 'x',
          body: '',
          parent_epic: null
        }
      ]
    }
    const result = boardResponseDecoder.validate(board)
    expect(result.type).toBe('ok')
    if (result.type === 'ok') expect(result.data.stories[0].parent_epic).toBeNull()
  })

  test('rejects a missing required field (story.state)', () => {
    const bad = {
      fetched_at: 'x',
      stories: [{ number: 1, title: 't', labels: [], assignees: [], created_at: 'x', body: '' }]
    }
    expect(boardResponseDecoder.validate(bad).type).toBe('error')
  })

  test('rejects a wrong-typed field (number sent as string)', () => {
    const bad = {
      fetched_at: 'x',
      stories: [
        {
          number: '1',
          title: 't',
          state: 'ready',
          labels: [],
          assignees: [],
          created_at: 'x',
          body: ''
        }
      ]
    }
    expect(boardResponseDecoder.validate(bad).type).toBe('error')
  })

  test('rejects an unknown state literal (the union is enforced)', () => {
    const bad = {
      fetched_at: 'x',
      stories: [
        {
          number: 1,
          title: 't',
          state: 'bogus',
          labels: [],
          assignees: [],
          created_at: 'x',
          body: ''
        }
      ]
    }
    expect(boardResponseDecoder.validate(bad).type).toBe('error')
  })

  test('rejects a non-object payload', () => {
    expect(boardResponseDecoder.validate(null).type).toBe('error')
    expect(boardResponseDecoder.validate('nope').type).toBe('error')
  })
})
