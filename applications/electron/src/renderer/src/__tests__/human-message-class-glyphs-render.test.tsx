import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../test/fixtures/seedMessages'

const INSIGHT_LEGACY_SEED: SeedMessage[] = [
  {
    from: 'dev:claude-code',
    text: 'a chat message to anchor scope',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'a legacy insight-class reasoning trace remapped to events',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'insight'
  }
]

const BPMUX_SEED: SeedMessage[] = [
  {
    from: 'dev:claude-code',
    text: 'a chat message to anchor known-roles',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'bpmux:system',
    text: 'a bpmux broadcast message',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'system'
  }
]

describe('human-side E2E #476 — read-class — message class glyphs (post-#1456 3-class remap)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('legacy insight-class seeded message remaps to events glyph (#1456 classOf legacy remap)', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)
    await seedMessages(apiBase, authToken, INSIGHT_LEGACY_SEED)

    rendered = renderApp()

    const eventsPill = await rendered.findByTestId('filter-pill-events', undefined, {
      timeout: 15_000
    })
    eventsPill.click()

    const allGlyphs = await rendered.findAllByTestId('message-class-glyph', undefined, {
      timeout: 15_000
    })
    const eventsGlyphs = allGlyphs.filter((el) => el.getAttribute('data-class') === 'events')
    expect(eventsGlyphs.length).toBeGreaterThan(0)

    const insightGlyphs = allGlyphs.filter((el) => el.getAttribute('data-class') === 'insight')
    expect(insightGlyphs.length).toBe(0)
  }, 20_000)

  it('legacy system-class seeded message remaps to bpmux glyph (#1456 classOf legacy remap)', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)
    await seedMessages(apiBase, authToken, BPMUX_SEED)

    rendered = renderApp()

    const filterPill = await rendered.findByTestId('filter-pill-bpmux', undefined, {
      timeout: 15_000
    })
    filterPill.click()

    const bpmuxGlyphs = await rendered.findAllByTestId('message-class-glyph', undefined, {
      timeout: 15_000
    })
    const bpmuxOnly = bpmuxGlyphs.filter((g) => g.getAttribute('data-class') === 'bpmux')
    expect(bpmuxOnly.length).toBe(1)

    const systemGlyphs = bpmuxGlyphs.filter((g) => g.getAttribute('data-class') === 'system')
    expect(systemGlyphs.length).toBe(0)
  }, 20_000)
})
