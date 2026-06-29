import { describe, expect, test } from 'vitest'
import { story } from './boardDecoder'

const validStory = {
  id: 2,
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

describe('story decoder', () => {
  test('decodes a fully-populated story (id, nested readiness/membership)', () => {
    const result = story.validate(validStory)
    expect(result.type).toBe('ok')
    if (result.type === 'ok') {
      expect(result.data.id).toBe(2)
      expect(result.data.readiness?.review_closed).toBe(false)
      expect(result.data.iteration_membership?.iteration_number).toBe(10)
      expect(result.data.rework_comment).toBeNull()
    }
  })

  test('decodes a minimal story (only required fields)', () => {
    const result = story.validate({
      id: 1,
      title: 'minimal story',
      state: 'ready',
      labels: ['Feature'],
      assignees: ['dev'],
      created_at: '2026-06-17T00:00:00Z',
      body: ''
    })
    expect(result.type).toBe('ok')
  })

  test('decodes parent_epic: null (serde Option::None) — the real-backend shape the old cast silently mis-typed', () => {
    const result = story.validate({
      id: 1,
      title: 't',
      state: 'ready',
      labels: [],
      assignees: [],
      created_at: 'x',
      body: '',
      parent_epic: null
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') expect(result.data.parent_epic).toBeNull()
  })

  test('rejects a missing required field (story.state)', () => {
    const bad = { id: 1, title: 't', labels: [], assignees: [], created_at: 'x', body: '' }
    expect(story.validate(bad).type).toBe('error')
  })

  test('rejects a wrong-typed field (id sent as string)', () => {
    const bad = {
      id: '1',
      title: 't',
      state: 'ready',
      labels: [],
      assignees: [],
      created_at: 'x',
      body: ''
    }
    expect(story.validate(bad).type).toBe('error')
  })

  test('rejects an unknown state literal (the union is enforced)', () => {
    const bad = {
      id: 1,
      title: 't',
      state: 'bogus',
      labels: [],
      assignees: [],
      created_at: 'x',
      body: ''
    }
    expect(story.validate(bad).type).toBe('error')
  })

  test('rejects a non-object payload', () => {
    expect(story.validate(null).type).toBe('error')
    expect(story.validate('nope').type).toBe('error')
  })
})
