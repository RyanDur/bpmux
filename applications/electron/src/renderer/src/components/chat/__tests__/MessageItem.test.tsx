import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import MessageItem from '../MessageItem'
import type { ChatMessage } from '@transport/types'

const baseMsg = (over: Partial<ChatMessage> = {}): ChatMessage => ({
  id: 'srv-1',
  from: 'owner',
  text: 'pick up #1604',
  ts: '2026-06-03T00:00:00.000Z',
  to: ['dev'],
  addressee: null,
  messageType: null,
  ...over
})

const render = (m: ChatMessage): string =>
  renderToStaticMarkup(
    <MessageItem
      m={m}
      grouped={false}
      presence={new Map()}
      mentionRoles={new Set()}
      onMention={() => {}}
    />
  )

describe('MessageItem — message render', () => {
  test('renders the message text and sender', () => {
    const html = render(baseMsg())
    expect(html).toContain('pick up #1604')
    expect(html).toContain('data-test="message-row"')
  })
})
