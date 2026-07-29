import { describe, expect, test } from 'vitest'
import { presentSyncError } from '../syncErrorCopy'

describe('the dialog shows the human clause, not the plumbing', () => {
  test('the ingest prefix is stripped', () => {
    expect(
      presentSyncError(
        'ingest: GitHub refused the token — check it in GitHub settings and reconnect'
      )
    ).toBe('GitHub refused the token — check it in GitHub settings and reconnect')
  })

  test('stacked mechanical prefixes all strip', () => {
    expect(presentSyncError('ingest: source unreachable: github not connected')).toBe(
      'github not connected'
    )
    expect(presentSyncError('ingest: fetch_body(42): boom')).toBe('boom')
    expect(presentSyncError('refresh_story(7): rate limited — retrying')).toBe(
      'rate limited — retrying'
    )
  })

  test('a message without plumbing passes through untouched', () => {
    expect(presentSyncError('rate limited — retrying in 60s')).toBe(
      'rate limited — retrying in 60s'
    )
  })
})
