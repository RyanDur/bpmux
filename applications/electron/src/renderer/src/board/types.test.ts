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
      ['needs-fixes', 'in-progress'],
      ['rejected', 'ready'],
      ['accepted', 'accepted']
    ])('open + state=%s → column %s', (state, column) => {
      expect(columnFor(state, false)).toBe(column)
    })
  })

  describe('#1146 slice 2b — closed-non-Accepted projects to RETIRED column', () => {
    test.each<[StoryState, ReturnType<typeof columnFor>]>([
      ['icebox', 'retired'],
      ['ready', 'retired'],
      ['in-progress', 'retired'],
      ['desk-check', 'retired'],
      ['delivered', 'retired'],
      ['ready-for-accept', 'retired'],
      ['needs-fixes', 'retired'],
      ['rejected', 'retired']
    ])(
      'closed + state=%s (non-Accepted) → RETIRED (decision-not-to-pursue)',
      (state, column) => {
        expect(columnFor(state, true)).toBe(column)
      }
    )

    test('closed + state=accepted stays in ACCEPTED (success-narrative preserved)', () => {
      expect(columnFor('accepted', true)).toBe('accepted')
    })
  })

  describe('isClosed defaults to false (back-compat for callers that pre-date #514)', () => {
    test('omitting isClosed argument behaves as open', () => {
      expect(columnFor('in-progress')).toBe('in-progress')
      expect(columnFor('rejected')).toBe('ready')
    })
  })

  describe('#514 slice 2 — needs-fixes splits from rejected for OPEN issues', () => {
    test('OPEN + needs-fixes is dev-iterating-on-feedback → IN PROGRESS, not READY', () => {
      // Designer-fail (fail_desk_check) leaves the story at needs-fixes
      // while dev keeps iterating. Buckets with active work, not the
      // pickup-able queue.
      expect(columnFor('needs-fixes', false)).toBe('in-progress')
    })
    test('OPEN + rejected stays in READY (owner-reject = re-pickup-able)', () => {
      expect(columnFor('rejected', false)).toBe('ready')
    })
  })
})
