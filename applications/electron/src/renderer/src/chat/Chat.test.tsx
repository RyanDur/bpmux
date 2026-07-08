import { afterEach, describe, expect, test } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
import Chat from './Chat'
import type { MessageFilter } from '../hooks/useMessageFilter'
import { dispatchState, resetStateForTest } from '../transport/state/store'
import { connected, connecting } from '../transport/state/actions'

const filter: MessageFilter = {
  visible: new Set(['chat']),
  toggle: () => {},
  reset: () => {},
  setOnly: () => {}
}

const renderChat = (): void => {
  render(
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
}

afterEach(() => {
  cleanup()
  resetStateForTest()
})

describe('Chat — reconnect warning reflects the session connection status (useConnectionStatus selector)', () => {
  test('a disconnected session shows the reconnecting warning', () => {
    renderChat()
    expect(screen.getByText('reconnecting…')).toBeTruthy()
  })

  test('a connecting session shows the connecting warning', () => {
    dispatchState(connecting())
    renderChat()
    expect(screen.getByText('connecting…')).toBeTruthy()
  })

  test('a connected session shows no reconnect warning', () => {
    dispatchState(connected())
    renderChat()
    expect(screen.queryByText(/^(connecting|reconnecting)…$/)).toBeNull()
  })
})
