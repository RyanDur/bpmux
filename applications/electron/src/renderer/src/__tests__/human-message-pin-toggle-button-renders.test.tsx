// #1306 sub-batch 4.1.A — migrated from
// tests/human-message-pin-toggle-button-renders.spec.ts.
// Recipe: docs/design/ci-migration/playwright-to-rtl-recipe.md.

import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../test/fixtures/seedMessages'

const SEED: SeedMessage[] = [
  {
    from: 'pm:claude-code',
    text: 'pinnable test message',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  }
]

describe('human-side E2E #476 — direct-action — message pin toggle', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'seeded chat message renders a pin-toggle button on hover affordance',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
      await seedMessages(apiBase, authToken, SEED)

      rendered = renderApp()

      await rendered.findByText(/pinnable test message/, undefined, { timeout: 15_000 })

      const pinButtons = rendered.container.querySelectorAll('[data-test="message-pin-toggle"]')
      expect(pinButtons.length).toBe(1)
      expect(pinButtons[0].getAttribute('aria-label')).toBe('Pin message')
    },
    20_000
  )
})
