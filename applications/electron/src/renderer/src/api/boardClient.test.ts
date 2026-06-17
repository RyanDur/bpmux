import { afterEach, describe, expect, test, vi } from 'vitest'

const { get } = vi.hoisted(() => ({ get: vi.fn() }))
vi.mock('./http', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./http')>()
  return { ...actual, http: { ...actual.http, get } }
})

import { getBoard } from './boardClient'
import { asyncSuccess } from '../lib/asyncResult'

const ctx = { apiBase: 'http://test.invalid', authToken: 'tok' }

describe('getBoard — decodes /api/board at the boundary', () => {
  afterEach(() => vi.clearAllMocks())

  test('a valid response decodes through to a BoardResponse success', async () => {
    get.mockReturnValue(asyncSuccess({ fetched_at: '2026-06-17T00:00:00Z', stories: [] }))
    const result = await getBoard(ctx).value
    expect(result.kind).toBe('success')
  })

  test('a wrong-shape response fails loud at the edge as a decode failure (not a deep crash)', async () => {
    get.mockReturnValue(asyncSuccess({ stories: 'not-an-array' }))
    const result = await getBoard(ctx).value
    expect(result.kind).toBe('failure')
    if (result.kind === 'failure') expect(result.reason.kind).toBe('decode')
  })
})
