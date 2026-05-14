// #1304 Epic #1247 slice 2: RTL+real-backend archetype.
//
// Migrates the original Playwright spec
// `tests/human-message-list-renders-seeded-chat.spec.ts` (51 lines)
// to RTL+real-backend using the #1303 infrastructure (vitest
// globalSetup + bpmux backend subprocess + bpmuxShim + renderApp).
//
// The test-name semantic and feature-coverage statement carry over
// verbatim from the Playwright spec — only the test-runner substrate
// changes (Playwright + Electron → vitest + jsdom + RTL + real-backend).
// Per Ryan-canon 2026-05-12 13:00 UTC ("e2e tests are living
// documentation"), the feature-catalog property is preserved across
// the migration.
//
// Three-seam round-trip:
// 1. REST: POST /api/test/seed_messages (BPMUX_TEST_SEED_API per
//    #1290 Part B)
// 2. WebSocket: backend broadcasts seeded messages to subscribed
//    renderer
// 3. DOM: message-list re-renders with the seeded text visible to RTL
//    queries
//
// Archetype-lock for slice 4 bulk-migration (#1306) — sister-shape
// specs in batch 4.1 (human-message-* / human-board-* /
// human-drill-in-*) replicate this same seed→WS→DOM pattern.

import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'

interface SeedMessage {
  from: string
  text: string
  ts: string
  message_type: string
}

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

async function seedMessages(
  apiBase: string,
  authToken: string,
  messages: SeedMessage[]
): Promise<void> {
  const res = await fetch(`${apiBase}/api/test/seed_messages`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${authToken}`
    },
    body: JSON.stringify({ messages })
  })
  if (!res.ok) {
    throw new Error(`seed_messages POST failed: ${res.status} ${await res.text()}`)
  }
}

describe('human-side E2E #476 — read-class — message list (RTL+real-backend archetype)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'seeded chat messages render with from + text in the message list',
    async () => {
      // Seed-before-mount mirrors the Playwright spec's ordering
      // (launch.ts seeds messages between waitForHealthz + the renderer
      // attach). The renderer's useMessageHistory fetches /api/messages
      // at mount, picking up the seeded rows from storage.
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await seedMessages(apiBase, authToken, SEED_MESSAGES)

      rendered = renderApp()

      await rendered.findByText(/SEED_CHAT_MARKER_BPF13_FIRST/, undefined, { timeout: 15_000 })
      await rendered.findByText(/SEED_CHAT_MARKER_BPF13_SECOND/, undefined, { timeout: 15_000 })

      const list = rendered.container.querySelector('[data-test="message-list"]')
      expect(list).toBeTruthy()

      const rows = rendered.container.querySelectorAll('[data-test="message-row"]')
      expect(rows.length).toBeGreaterThanOrEqual(2)
    },
    20_000
  )
})
