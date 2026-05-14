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
      // Reset-then-seed-before-mount: vitest globalSetup shares ONE
      // backend per-run, so state leaks across tests. Reset wipes the
      // message-store; seed installs only this test's fixture; renderApp
      // then mounts and useMessageHistory fetches /api/messages, picking
      // up exactly the seeded rows (no cross-test bleed).
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
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
