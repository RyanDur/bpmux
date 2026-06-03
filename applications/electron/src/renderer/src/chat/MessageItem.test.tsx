import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import MessageItem from './MessageItem'
import type { ChatMessage } from '../transport/types'

const baseMsg = (over: Partial<ChatMessage> = {}): ChatMessage => ({
  id: 'srv-1',
  from: 'ryan',
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

describe('#1604 MessageItem — delivered/seen receipt', () => {
  test('renders no receipt when the message carries no delivery status', () => {
    expect(render(baseMsg())).not.toContain('data-test="message-receipt"')
  })

  test('delivered renders a receipt that names the recipient and labels it for screen readers', () => {
    const html = render(baseMsg({ deliveryStatus: { dev: 'delivered' } }))
    expect(html).toContain('data-status="delivered"')
    expect(html).toContain('data-recipient="dev"')
    expect(html).toContain('message-receipt--delivered')
    expect(html).toContain('delivered to dev')
  })

  test('seen renders a glanceably DISTINCT receipt from delivered', () => {
    const delivered = render(baseMsg({ deliveryStatus: { dev: 'delivered' } }))
    const seen = render(baseMsg({ deliveryStatus: { dev: 'seen' } }))
    expect(seen).toContain('data-status="seen"')
    expect(seen).toContain('message-receipt--seen')
    expect(seen).toContain('seen by dev')
    expect(seen).not.toBe(delivered)
  })

  test('each recipient of a multi-addressed message gets its own receipt', () => {
    const html = render(
      baseMsg({ to: ['dev', 'pm'], deliveryStatus: { dev: 'seen', pm: 'delivered' } })
    )
    expect(html).toContain('data-recipient="dev"')
    expect(html).toContain('data-recipient="pm"')
  })
})
