import { afterEach, describe, expect, test } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import MessageList from './MessageList'
import { classOf, type MessageClass } from '../hooks/useMessageFilter'
import type { ChatMessage } from '../transport/types'

const VISIBLE: ReadonlySet<MessageClass> = new Set([classOf(undefined)])

const msg = (id: string, ts: string): ChatMessage => ({
  id,
  from: 'dev',
  text: `body-${id}`,
  ts
})

const enteringRows = (root: HTMLElement): string[] =>
  Array.from(root.querySelectorAll('[data-test="message-row"][data-entering="true"]')).map(
    (el) => el.textContent ?? ''
  )

describe('#1487 message rise-from-below — gated to genuinely-new arrivals', () => {
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
