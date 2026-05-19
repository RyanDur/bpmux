import { afterEach, describe, expect, inject, it } from 'vitest'
import { screen } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../test/fixtures/seedMessages'

const INSIGHT_SEED: SeedMessage[] = [
  {
    from: 'dev:claude-code',
    text: 'an insight-class reasoning trace',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'insight'
  }
]

const SYSTEM_SEED: SeedMessage[] = [
  {
    from: 'dev:claude-code',
    text: 'a chat message to anchor known-roles',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'bpmux:system',
    text: 'a system event message',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'system'
  }
]

describe('human-side E2E #476 — read-class — message class glyphs', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'insight-class seeded message renders the insight class glyph',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
      await seedMessages(apiBase, authToken, INSIGHT_SEED)

      rendered = renderApp()

      await rendered.findByText(/reasoning trace/, undefined, { timeout: 15_000 })

      const allGlyphs = screen.queryAllByTestId('message-class-glyph')
      const insightGlyphs = allGlyphs.filter((el) => el.getAttribute('data-class') === 'insight')
      expect(insightGlyphs.length).toBe(1)

      const chatGlyphs = allGlyphs.filter((el) => el.getAttribute('data-class') === 'chat')
      expect(chatGlyphs.length).toBe(0)
    },
    20_000
  )

  it(
    'system-class seeded message renders the system class glyph',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
      await seedMessages(apiBase, authToken, SYSTEM_SEED)

      rendered = renderApp()

      const filterPill = await rendered.findByTestId('filter-pill-system', undefined, {
        timeout: 15_000
      })
      filterPill.click()

      const systemGlyphs = await rendered.findAllByTestId('message-class-glyph', undefined, {
        timeout: 15_000
      })
      const systemOnly = systemGlyphs.filter((g) => g.getAttribute('data-class') === 'system')
      expect(systemOnly.length).toBe(1)
    },
    20_000
  )
})
