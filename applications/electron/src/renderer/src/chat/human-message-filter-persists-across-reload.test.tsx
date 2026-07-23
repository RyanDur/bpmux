import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { resetMessages } from '../../../../test/fixtures/seedMessages'
import { humanMessage } from './humanMessage.page'

describe('message filter (3-class mute-toggle)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
    window.localStorage.clear()
  })

  it('toggling events-class on persists across reload', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)

    rendered = renderApp()

    expect(await humanMessage.filterPills()).toBeTruthy()

    const eventsPill = await humanMessage.pill('events')
    expect(humanMessage.pillChecked('events')).toBe(false)

    await humanMessage.togglePill(eventsPill)
    await waitFor(() => expect(humanMessage.pillChecked('events')).toBe(true))

    rendered.cleanup()
    rendered = renderApp()

    await humanMessage.pill('events')
    expect(humanMessage.pillChecked('events')).toBe(true)
  }, 20_000)

  it('toggling chat-class off persists across reload', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)

    rendered = renderApp()

    const chatPill = await humanMessage.pill('chat')
    expect(humanMessage.pillChecked('chat')).toBe(true)

    await humanMessage.togglePill(chatPill)
    await waitFor(() => expect(humanMessage.pillChecked('chat')).toBe(false))

    rendered.cleanup()
    rendered = renderApp()

    await humanMessage.pill('chat')
    expect(humanMessage.pillChecked('chat')).toBe(false)
  }, 20_000)
})
