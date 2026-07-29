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

describe('history hydration rides the delta family', () => {
  it('a history state delta lands as historyLoaded, never in the mirror', () => {
    const dispatch = vi.fn()
    dispatchServerFrame(
      JSON.stringify({
        type: 'message',
        from: 'bpmux',
        text: '',
        ts: '2026-07-29T12:00:00Z',
        message_type: 'state-delta',
        state_delta: {
          domain: 'history',
          messages: [{ from: 'dev', text: 'old chat', ts: '2026-07-28T12:00:00Z' }]
        }
      }),
      dispatch,
      () => 1
    )
    expect(dispatch).toHaveBeenCalledTimes(1)
    const action = dispatch.mock.calls[0][0]
    expect(action.type).toEqual('state/historyLoaded')
    expect(action.history[0].text).toEqual('old chat')
  })
})
