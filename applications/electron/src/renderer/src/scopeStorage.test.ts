import { afterEach, describe, expect, test } from 'vitest'
import { RENDERER_IDENTITY } from './state'
import { readScope, writeScope } from './scopeStorage'

const STORAGE_KEY = `bpmux-scope:${RENDERER_IDENTITY}`

afterEach(() => {
  window.localStorage.clear()
})

describe('scopeStorage', () => {
  test('round-trips an all-view scope', () => {
    writeScope({ kind: 'all' })
    expect(readScope()).toEqual({ kind: 'all' })
  })

  test('round-trips a broadcast scope', () => {
    writeScope({ kind: 'broadcast' })
    expect(readScope()).toEqual({ kind: 'broadcast' })
  })

  test('round-trips a scoped recipient list', () => {
    writeScope({ kind: 'scoped', value: ['pm', 'designer'] })
    expect(readScope()).toEqual({ kind: 'scoped', value: ['pm', 'designer'] })
  })

  test('returns null when nothing is stored', () => {
    expect(readScope()).toBeNull()
  })

  test('returns null for malformed JSON', () => {
    window.localStorage.setItem(STORAGE_KEY, '{not valid json')
    expect(readScope()).toBeNull()
  })

  test('returns null for an unknown kind', () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ kind: 'mystery' }))
    expect(readScope()).toBeNull()
  })

  test('rejects a scoped value with no usable strings', () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ kind: 'scoped', value: [] }))
    expect(readScope()).toBeNull()
  })

  test('filters non-string entries out of a scoped value', () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ kind: 'scoped', value: ['pm', 7, null, 'dev'] })
    )
    expect(readScope()).toEqual({ kind: 'scoped', value: ['pm', 'dev'] })
  })
})
