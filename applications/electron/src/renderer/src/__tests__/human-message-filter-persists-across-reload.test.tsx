import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { resetMessages } from '../../../../test/fixtures/seedMessages'

describe('human-side E2E #476 — persistence-class — message filter (#1456 3-class mute-toggle)', () => {
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

    const filterPills = await rendered.findByTestId('filter-pills', undefined, {
      timeout: 15_000
    })
    expect(filterPills).toBeTruthy()

    const eventsPill = await rendered.findByTestId('filter-pill-events')
    const eventsInput = eventsPill.querySelector('input[type="checkbox"]') as HTMLInputElement
    expect(eventsInput.checked).toBe(false)

    eventsPill.click()
    await waitFor(() =>
      expect((eventsPill.querySelector('input[type="checkbox"]') as HTMLInputElement).checked).toBe(
        true
      )
    )

    rendered.cleanup()
    rendered = renderApp()

    const eventsAfterReload = await rendered.findByTestId('filter-pill-events', undefined, {
      timeout: 15_000
    })
    expect(
      (eventsAfterReload.querySelector('input[type="checkbox"]') as HTMLInputElement).checked
    ).toBe(true)
  }, 20_000)

  it('toggling chat-class off persists across reload', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)

    rendered = renderApp()

    const chatPill = await rendered.findByTestId('filter-pill-chat', undefined, {
      timeout: 15_000
    })
    expect((chatPill.querySelector('input[type="checkbox"]') as HTMLInputElement).checked).toBe(
      true
    )

    chatPill.click()
    await waitFor(() =>
      expect((chatPill.querySelector('input[type="checkbox"]') as HTMLInputElement).checked).toBe(
        false
      )
    )

    rendered.cleanup()
    rendered = renderApp()

    const chatAfterReload = await rendered.findByTestId('filter-pill-chat', undefined, {
      timeout: 15_000
    })
    expect(
      (chatAfterReload.querySelector('input[type="checkbox"]') as HTMLInputElement).checked
    ).toBe(false)
  }, 20_000)
})
