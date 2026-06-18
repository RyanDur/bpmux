import { describe, expect, test } from 'vitest'
import { isMessageInScope } from './scope'
import type { ChatMessage } from '../transport/types'

const msg = (from: string, to: string[] | null): ChatMessage =>
  ({ from, to, text: 't', ts: '2026-06-14T00:00:00Z' }) as ChatMessage

describe('isMessageInScope — owner-omniscience scoping (#1733)', () => {
  test('the owner all-conversations view renders every message, regardless of --to', () => {
    expect(
      isMessageInScope(msg('dev:claude-code', ['pm', 'designer']), null, 'ryan:human', true)
    ).toBe(true)
  })

  test('the owner GROUP room stays scoped: an out-of-group message does NOT leak in', () => {
    expect(
      isMessageInScope(
        msg('dev:claude-code', ['dev', 'pm']),
        ['designer', 'dev'],
        'ryan:human',
        false
      )
    ).toBe(false)
  })

  test('the owner GROUP room DOES render its own in-group messages', () => {
    expect(
      isMessageInScope(
        msg('designer:claude-code', ['designer', 'dev']),
        ['designer', 'dev'],
        'ryan:human',
        false
      )
    ).toBe(true)
  })
})
