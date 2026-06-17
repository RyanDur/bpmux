import { afterEach, describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Chat from './Chat'
import type { MessageFilter } from '../hooks/useMessageFilter'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { connected, connecting } from '../transport/session/actions'

const filter: MessageFilter = {
  visible: new Set(['chat']),
  toggle: () => {},
  reset: () => {},
  setOnly: () => {}
}

const renderChat = (): string =>
  renderToStaticMarkup(
    <Chat
      messages={[]}
      recipient={null}
      allView={false}
      activeKey={null}
      terminals={[]}
      filter={filter}
      metricsOpen={false}
      onCloseMetrics={() => {}}
      apiBase={undefined}
      authToken={undefined}
      onSend={() => {}}
    />
  )

afterEach(() => resetSessionForTest())

describe('Chat — reconnect warning reflects the session connection status (useConnectionStatus selector)', () => {
  test('a disconnected session shows the reconnecting warning', () => {
    const html = renderChat()
    expect(html).toContain('data-test="chat-warning"')
    expect(html).toContain('reconnecting…')
  })

  test('a connecting session shows the connecting warning', () => {
    dispatchSession(connecting())
    const html = renderChat()
    expect(html).toContain('data-test="chat-warning"')
    expect(html).toContain('connecting…')
  })

  test('a connected session shows no reconnect warning', () => {
    dispatchSession(connected())
    const html = renderChat()
    expect(html).not.toContain('data-test="chat-warning"')
  })
})
