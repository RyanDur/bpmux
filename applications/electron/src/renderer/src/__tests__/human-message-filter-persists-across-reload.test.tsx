import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { resetMessages } from '../../../../test/fixtures/seedMessages'

describe('human-side E2E #476 — persistence-class — message filter', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
    window.localStorage.clear()
  })

  it('toggling activity-class on persists across reload', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)

    rendered = renderApp()

    const filterPills = await rendered.findByTestId('filter-pills', undefined, {
      timeout: 15_000
    })
    expect(filterPills).toBeTruthy()

    const activityPill = await rendered.findByTestId('filter-pill-activity')
    expect(activityPill.getAttribute('aria-pressed')).toBe('false')

    activityPill.click()
    await waitFor(() => expect(activityPill.getAttribute('aria-pressed')).toBe('true'))

    rendered.cleanup()
    rendered = renderApp()

    const activityAfterReload = await rendered.findByTestId('filter-pill-activity', undefined, {
      timeout: 15_000
    })
    expect(activityAfterReload.getAttribute('aria-pressed')).toBe('true')
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
    expect(chatPill.getAttribute('aria-pressed')).toBe('true')

    chatPill.click()
    await waitFor(() => expect(chatPill.getAttribute('aria-pressed')).toBe('false'))

    rendered.cleanup()
    rendered = renderApp()

    const chatAfterReload = await rendered.findByTestId('filter-pill-chat', undefined, {
      timeout: 15_000
    })
    expect(chatAfterReload.getAttribute('aria-pressed')).toBe('false')
  }, 20_000)
})
