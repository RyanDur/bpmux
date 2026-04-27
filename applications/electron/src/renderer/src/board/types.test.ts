import { describe, expect, test } from 'vitest'
import { columnFor, type StoryState } from './types'

describe('#514 columnFor — closed-state-aware bucketing', () => {
  describe('open issues route to active-pipeline columns (no regression vs pre-#514)', () => {
    test.each<[StoryState, ReturnType<typeof columnFor>]>([
      ['icebox', 'icebox'],
      ['ready', 'ready'],
      ['in-progress', 'in-progress'],
      ['desk-check', 'desk-check'],
      ['delivered', 'delivered'],
      ['ready-for-accept', 'delivered'],
      ['needs-fixes', 'ready'],
      ['rejected', 'ready'],
      ['accepted', 'accepted']
    ])('open + state=%s → column %s', (state, column) => {
      expect(columnFor(state, false)).toBe(column)
    })
  })

  describe('closed issues route out of active-pipeline regardless of last-domain-state (#464 preserved internally)', () => {
    test.each<[StoryState, ReturnType<typeof columnFor>]>([
      ['ready', 'accepted'],
      ['in-progress', 'accepted'],
      ['desk-check', 'accepted'],
      ['delivered', 'accepted'],
      ['ready-for-accept', 'accepted'],
      ['needs-fixes', 'accepted'],
      ['rejected', 'accepted'],
      ['accepted', 'accepted']
    ])('closed + state=%s → ACCEPTED column (visual closure)', (state, column) => {
      expect(columnFor(state, true)).toBe(column)
    })

    test('closed + state=icebox → ICEBOX (legitimate historical, not closure-as-acceptance)', () => {
      expect(columnFor('icebox', true)).toBe('icebox')
    })
  })

  describe('isClosed defaults to false (back-compat for callers that pre-date #514)', () => {
    test('omitting isClosed argument behaves as open', () => {
      expect(columnFor('in-progress')).toBe('in-progress')
      expect(columnFor('rejected')).toBe('ready')
    })
  })
})
