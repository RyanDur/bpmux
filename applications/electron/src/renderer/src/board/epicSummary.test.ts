import { describe, expect, test } from 'vitest'
import {
  breakdownFor,
  extractDescription,
  isOpenByDefault,
  progressDenominatorFor,
  sortEpics,
  stripEpicPrefix
} from './epicSummary'
import type { Story, StoryState } from './types'
import { asStoryId } from './types'

let seq = 1
const story = (over: Partial<Story>): Story => ({
  id: asStoryId(seq++),
  title: 'an epic',
  state: 'ready',
  labels: [],
  assignees: [],
  created_at: '2026-06-16T00:00:00Z',
  body: '',
  ...over
})

const child = (epicNumber: number, state: StoryState): Story =>
  story({ parent_epic: asStoryId(epicNumber), state })

describe('epicSummary', () => {
  test('breakdownFor groups child states (delivered+ready-for-accept, in-progress+needs-fixes, ready+rejected) and scopes to the epic', () => {
    const b = breakdownFor(story({ id: asStoryId(100) }), [
      child(100, 'accepted'),
      child(100, 'delivered'),
      child(100, 'ready-for-accept'),
      child(100, 'in-progress'),
      child(100, 'needs-fixes'),
      child(100, 'desk-check'),
      child(100, 'ready'),
      child(100, 'rejected'),
      child(100, 'icebox'),
      child(100, 'retired'),
      child(999, 'accepted')
    ])
    expect(b.total).toBe(10)
    expect(b.accepted).toBe(1)
    expect(b.delivered).toBe(2)
    expect(b.inProgress).toBe(2)
    expect(b.deskCheck).toBe(1)
    expect(b.ready).toBe(2)
    expect(b.icebox).toBe(1)
    expect(b.retired).toBe(1)
  })

  test('progressDenominatorFor excludes retired (scope-removed children)', () => {
    const b = breakdownFor(story({ id: asStoryId(100) }), [
      child(100, 'accepted'),
      child(100, 'in-progress'),
      child(100, 'retired')
    ])
    expect(progressDenominatorFor(b)).toBe(2)
  })

  test('extractDescription prefers the User job section, else the first paragraph', () => {
    expect(extractDescription('## User job\n\nthe job\n\n## Why\nreason')).toBe('the job')
    expect(extractDescription('just a description\n\nmore')).toBe('just a description')
    expect(extractDescription('')).toBe('')
  })

  test('stripEpicPrefix removes a leading [Epic] tag, leaves plain titles', () => {
    expect(stripEpicPrefix('[Epic] Roadmap')).toBe('Roadmap')
    expect(stripEpicPrefix('Roadmap')).toBe('Roadmap')
  })

  test('isOpenByDefault: open unless the epic is accepted or iceboxed', () => {
    expect(isOpenByDefault('in-progress')).toBe(true)
    expect(isOpenByDefault('accepted')).toBe(false)
    expect(isOpenByDefault('icebox')).toBe(false)
  })

  test('sortEpics by progress orders the more-complete epic first', () => {
    const a = story({ id: asStoryId(1) })
    const b = story({ id: asStoryId(2) })
    const stories = [
      a,
      b,
      child(1, 'accepted'),
      child(1, 'accepted'),
      child(2, 'accepted'),
      child(2, 'in-progress')
    ]
    expect(sortEpics([b, a], 'progress', stories).map((epic) => epic.id)).toEqual([1, 2])
  })
})
