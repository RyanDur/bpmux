import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../../../test/fixtures/seedMessages'
import { humanMessage } from './humanMessage.page'

const SEED: SeedMessage[] = [
  {
    from: 'dev:claude-code',
    text: 'first dev-authored message — establishes dev as known role',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'hey @dev — please look at this',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  }
]

describe('message @mention pill', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('seeded chat message with @<known-role> renders the role as a clickable pill', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)
    await seedMessages(apiBase, authToken, SEED)

    rendered = renderApp()

    await humanMessage.waitForText(/please look at this/)

    const mentions = await humanMessage.mentions()
    expect(mentions.length).toBeGreaterThanOrEqual(1)
    expect(humanMessage.mentionText(mentions[0])).toContain('@dev')
  }, 20_000)
})
