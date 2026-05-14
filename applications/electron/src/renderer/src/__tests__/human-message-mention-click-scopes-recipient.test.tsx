// #1335 sub-batch 4.1.B — migrated from
// tests/human-message-mention-click-scopes-recipient.spec.ts.
// Recipe: docs/design/ci-migration/playwright-to-rtl-recipe.md.

import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  seedRoster,
  type SeedMessage
} from '../../../../test/fixtures/seedMessages'

const SEED: SeedMessage[] = [
  {
    from: 'dev:claude-code',
    text: 'first dev-authored message — establishes dev as known role',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'pinging @dev — please look at this',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  }
]

describe('human-side E2E #476 — direct-action — message @mention click', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'clicking an @<role> pill scopes the chat recipient to that role',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
      await seedMessages(apiBase, authToken, SEED)
      await seedRoster(apiBase, authToken, ['dev:claude-code', 'pm:claude-code'])

      rendered = renderApp()

      await rendered.findByText(/please look at this/, undefined, { timeout: 15_000 })

      // sidebar-direct-dev surfaces after the roster state-snapshot
      // propagates via WS from the seeded orchestrator RosterStore.
      const devButton = await rendered.findByTestId('sidebar-direct-dev', undefined, {
        timeout: 15_000
      })
      const broadcastButton = rendered.getByTestId('sidebar-broadcast')
      expect(broadcastButton.getAttribute('aria-current')).toBe('true')

      // Click the mention pill — should scope recipient to dev
      const mention = (await rendered.findAllByTestId('message-mention'))[0]
      mention.click()

      await waitFor(
        () => {
          expect(devButton.getAttribute('aria-current')).toBe('true')
          expect(broadcastButton.getAttribute('aria-current')).toBeNull()
        },
        { timeout: 15_000 }
      )
    },
    30_000
  )
})
