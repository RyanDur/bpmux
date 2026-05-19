import { afterEach, describe, expect, inject, it } from 'vitest'
import { screen } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../test/fixtures/seedMessages'

const SEED_MESSAGES: SeedMessage[] = [
  {
    from: 'ryan:human',
    text: 'SEED_CHAT_MARKER_BPF13_FIRST',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'SEED_CHAT_MARKER_BPF13_SECOND',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  }
]

describe('human-side E2E #476 — read-class — message list (RTL+real-backend archetype)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'seeded chat messages render with from + text in the message list',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
      await seedMessages(apiBase, authToken, SEED_MESSAGES)

      rendered = renderApp()

      await rendered.findByText(/SEED_CHAT_MARKER_BPF13_FIRST/, undefined, { timeout: 15_000 })
      await rendered.findByText(/SEED_CHAT_MARKER_BPF13_SECOND/, undefined, { timeout: 15_000 })

      const list = screen.getByTestId('message-list')
      expect(list).toBeTruthy()

      const rows = screen.getAllByTestId('message-row')
      expect(rows.length).toBeGreaterThanOrEqual(2)
    },
    20_000
  )
})
