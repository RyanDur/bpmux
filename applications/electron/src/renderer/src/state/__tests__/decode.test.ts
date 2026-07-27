import { describe, expect, it, vi } from 'vitest'
import { dispatchServerFrame } from '../decode'

describe('legacy frame tolerance', () => {
  it('a state-snapshot frame from an old server is dropped, never rendered', () => {
    const dispatch = vi.fn()
    dispatchServerFrame(
      JSON.stringify({
        type: 'message',
        from: 'bpmux',
        text: '',
        ts: '2026-07-26T12:00:00Z',
        message_type: 'state-snapshot'
      }),
      dispatch,
      () => 1
    )
    expect(dispatch).not.toHaveBeenCalled()
  })
})
