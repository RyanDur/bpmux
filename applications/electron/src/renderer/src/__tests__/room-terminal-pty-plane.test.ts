import { describe, test, expect, beforeEach } from 'vitest'

import {
  __resetWsForTest,
  __deliverServerFrameForTest,
  subscribeWsPtyOutput,
  getWsSnapshot
} from '../transport/wsService'

describe('#1466 SLICE-1 watch — PTY output is a non-persisted plane (falsifier e)', () => {
  beforeEach(() => __resetWsForTest())

  test('pty_output streams to the terminal-id subscriber and is NOT persisted as message history', () => {
    const received: string[] = []
    const unsub = subscribeWsPtyOutput('dev', (d) => received.push(d))
    __deliverServerFrameForTest(
      JSON.stringify({ type: 'pty_output', terminal_id: 'dev', data: 'aGVsbG8=' })
    )
    expect(received).toEqual(['aGVsbG8='])
    expect(getWsSnapshot().messages).toEqual([])
    unsub()
  })

  test('a delivered chat message DOES persist — the pty plane is the exception, not the rule', () => {
    __deliverServerFrameForTest(
      JSON.stringify({ type: 'message', from: 'pm', text: 'hi', ts: '2026-05-30T00:00:00Z' })
    )
    expect(getWsSnapshot().messages.map((m) => m.text)).toEqual(['hi'])
  })

  test('pty_output for another terminal-id does not reach this terminal subscriber', () => {
    const received: string[] = []
    subscribeWsPtyOutput('dev', (d) => received.push(d))
    __deliverServerFrameForTest(
      JSON.stringify({ type: 'pty_output', terminal_id: 'pm', data: 'eA==' })
    )
    expect(received).toEqual([])
  })

  test('unsubscribe stops further pty_output delivery', () => {
    const received: string[] = []
    const unsub = subscribeWsPtyOutput('dev', (d) => received.push(d))
    unsub()
    __deliverServerFrameForTest(
      JSON.stringify({ type: 'pty_output', terminal_id: 'dev', data: 'eQ==' })
    )
    expect(received).toEqual([])
  })
})
