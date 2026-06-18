import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  seedRoster,
  type SeedMessage
} from '../../../../test/fixtures/seedMessages'
import { humanMessage } from '../chat/humanMessage.page'

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

  it('clicking an @<role> pill scopes the chat recipient to that role', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)
    await seedMessages(apiBase, authToken, SEED)
    await seedRoster(apiBase, authToken, ['dev:claude-code', 'pm:claude-code'])

    rendered = renderApp()

    await humanMessage.waitForText(/please look at this/)

    const initialAll = await humanMessage.sidebarAll()
    expect(initialAll.getAttribute('aria-current')).toBe('true')

    await humanMessage.mentions()

    await humanMessage.clickFirstMention()

    await waitFor(
      () => {
        expect(humanMessage.directChannelCurrent('dev')).toBe('true')
        expect(humanMessage.allChannelCurrent()).toBeNull()
      },
      { timeout: 15_000 }
    )
  }, 30_000)
})
