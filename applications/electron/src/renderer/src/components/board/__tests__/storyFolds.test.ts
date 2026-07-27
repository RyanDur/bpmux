import { describe, expect, test } from 'vitest'
import { orderByIds, removeById, upsertById } from '../storyFolds'
import type { Story } from '../types'

const story = (id: number, state = 'ready'): Story => ({ id, title: `s${id}`, state }) as Story

describe('granular story folds', () => {
  test('an upsert replaces the story in place, keeping its position', () => {
    const next = upsertById([story(1), story(2, 'in-progress'), story(3)], story(2, 'acceptance'))
    expect(next.map((s) => s.id)).toEqual([1, 2, 3])
    expect(next[1].state).toBe('acceptance')
  })

  test('an upsert of an unknown story appends it', () => {
    const next = upsertById([story(1)], story(9))
    expect(next.map((s) => s.id)).toEqual([1, 9])
  })

  test('a removal drops exactly the named story', () => {
    const next = removeById([story(1), story(2)], 1)
    expect(next.map((s) => s.id)).toEqual([2])
  })

  test('an order frame rearranges to the server truth', () => {
    const next = orderByIds([story(1), story(2), story(3)], [3, 1, 2])
    expect(next.map((s) => s.id)).toEqual([3, 1, 2])
  })

  test('stories the order frame does not know keep their relative place at the tail', () => {
    const next = orderByIds([story(1), story(7), story(2)], [2, 1])
    expect(next.map((s) => s.id)).toEqual([2, 1, 7])
  })

  test('order ids for stories the client has not seen yet are skipped, never holes', () => {
    const next = orderByIds([story(1)], [5, 1])
    expect(next.map((s) => s.id)).toEqual([1])
  })
})
