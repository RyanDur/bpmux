import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../test/fixtures/seedMessages'

const SEED: SeedMessage[] = [
  {
    from: 'pm:claude-code',
    text: 'reactable test message',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  }
]

describe('human-side E2E #476 — direct-action — message reaction picker', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'reaction-add → pick emoji → pill renders on the message (full cycle)',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
      await seedMessages(apiBase, authToken, SEED)

      rendered = renderApp()

      await rendered.findByText(/reactable test message/, undefined, { timeout: 15_000 })

      const addButton = (await rendered.findAllByTestId('reaction-add-first'))[0] as HTMLElement
      expect(addButton.getAttribute('aria-expanded')).toBe('false')

      addButton.click()
      await waitFor(() => expect(addButton.getAttribute('aria-expanded')).toBe('true'))

      const picker = await rendered.findByTestId('reaction-picker', undefined, {
        timeout: 5_000
      })
      expect(picker).toBeTruthy()

      const thumbsUp = await rendered.findByTestId('reaction-picker-👍', undefined, {
        timeout: 5_000
      })
      ;(thumbsUp as HTMLElement).click()

      await waitFor(() => {
        expect(rendered!.container.querySelectorAll('[data-test="reaction-picker"]').length).toBe(0)
      })

      await rendered.findByTestId('reaction-pill-👍', undefined, { timeout: 15_000 })
    },
    20_000
  )
})
