import { afterEach, describe, expect, test } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import MessageList from './MessageList'
import { classOf, type MessageClass } from '../chat/useMessageFilter'
import { MESSAGE_TYPE } from '../transport/messageType'
import type { ChatMessage, RosterMemberWire } from '../transport/types'
import { dispatchState, resetStateForTest } from '../state/store'
import { mirrorDeltaReceived } from '../state/actions'

const VISIBLE: ReadonlySet<MessageClass> = new Set([classOf(undefined)])

const msg = (id: string, ts: string): ChatMessage => ({
  id,
  from: 'dev',
  text: `body-${id}`,
  ts
})

const rosterMember = (name: string, presence: RosterMemberWire['presence']): RosterMemberWire => ({
  routing_id: `rt-${name}`,
  name,
  identity: `${name}:claude-code`,
  role: 'Dev',
  presence
})

const pushRoster = (members: RosterMemberWire[]): void =>
  dispatchState(mirrorDeltaReceived({ domain: 'roster', members }, '2026-01-01T00:00:00Z'))

const avatarStatus = (root: HTMLElement): HTMLElement | null =>
  root.querySelector('[data-test="avatar-tooltip-status"]')

const enteringRows = (root: HTMLElement): string[] =>
  Array.from(root.querySelectorAll('[data-test="message-row"][data-entering="true"]')).map(
    (el) => el.textContent ?? ''
  )

describe('message rise-from-below — gated to genuinely-new arrivals', () => {
  afterEach(cleanup)

  test('a freshly-loaded channel marks NO row entering (no cascade on channel-open)', () => {
    const { container } = render(
      <MessageList
        messages={[
          msg('a', '2026-06-05T10:00:00Z'),
          msg('b', '2026-06-05T10:01:00Z'),
          msg('c', '2026-06-05T10:02:00Z')
        ]}
        visibleClasses={VISIBLE}
        allView
      />
    )
    expect(enteringRows(container)).toEqual([])
  })

  test('a newly-arrived message (same channel) is marked entering; prior messages are not', () => {
    const initial = [msg('a', '2026-06-05T10:00:00Z'), msg('b', '2026-06-05T10:01:00Z')]
    const { container, rerender } = render(
      <MessageList messages={initial} visibleClasses={VISIBLE} allView />
    )
    expect(enteringRows(container)).toEqual([])

    rerender(
      <MessageList
        messages={[...initial, msg('c', '2026-06-05T10:02:00Z')]}
        visibleClasses={VISIBLE}
        allView
      />
    )
    const entering = enteringRows(container)
    expect(entering).toHaveLength(1)
    expect(entering[0]).toContain('body-c')
  })

  test('an older message loaded via scroll-up (older ts) is NOT marked entering', () => {
    const initial = [msg('a', '2026-06-05T10:00:00Z'), msg('b', '2026-06-05T10:01:00Z')]
    const { container, rerender } = render(
      <MessageList messages={initial} visibleClasses={VISIBLE} allView />
    )

    rerender(
      <MessageList
        messages={[msg('old', '2026-06-05T09:00:00Z'), ...initial]}
        visibleClasses={VISIBLE}
        allView
      />
    )
    expect(enteringRows(container)).toEqual([])
  })
})

describe('chat presence reads the single /api/presence roster source — not the legacy activity-message keyword-scan', () => {
  afterEach(() => {
    cleanup()
    resetStateForTest()
  })

  test('a chat sender avatar reflects the ROSTER presence, not a seeded offline', () => {
    pushRoster([rosterMember('dev', 'Busy')])
    const { container } = render(
      <MessageList
        messages={[msg('m1', '2026-06-05T10:00:00Z')]}
        visibleClasses={VISIBLE}
        allView
      />
    )
    const status = avatarStatus(container)
    expect(status?.textContent).toBe('busy')
    expect(status?.className).toContain('avatar-tooltip-status--busy')
  })

  test('an activity-message keyword does NOT drive chat presence — the roster wins (keyword-scan retired)', () => {
    pushRoster([rosterMember('dev', 'Busy')])
    const { container } = render(
      <MessageList
        messages={[
          {
            id: 'act',
            from: 'dev',
            text: 'idle',
            ts: '2026-06-05T09:59:00Z',
            messageType: MESSAGE_TYPE.Activity
          },
          msg('m1', '2026-06-05T10:00:00Z')
        ]}
        visibleClasses={VISIBLE}
        allView
      />
    )
    const status = avatarStatus(container)
    expect(status?.textContent).toBe('busy')
    expect(status?.textContent).not.toBe('idle')
  })
})
