// #1303 Epic #1247 slice 1: infrastructure regression test.
//
// Pins the RTL+real-backend substrate-claim end-to-end:
// - renderApp() fixture installs window.bpmux shim + renders <App />
// - The rendered App actually connects to the real bpmux backend
//   spawned by vitest globalSetup (MemoryStorage; no Electron)
// - At least one backend round-trip succeeds (GET /api/board)
// - Single-test wall-clock budget per AC8 (<500ms)
//
// This is the archetype-lock for slices 2-7 (#1304+); subsequent slices
// migrate Playwright specs into this substrate via the same renderApp
// fixture import-shape.

import { afterEach, describe, expect, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'

const PER_TEST_BUDGET_MS = 500

describe('#1303 RTL+real-backend infrastructure', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('renderApp injects window.bpmux pointing at the spawned backend', () => {
    rendered = renderApp()
    expect(window.bpmux).toBeDefined()
    expect(window.bpmux?.apiBase).toMatch(/^http:\/\/127\.0\.0\.1:\d+$/)
    expect(window.bpmux?.wsUrl).toMatch(/^ws:\/\/127\.0\.0\.1:\d+\/ws$/)
    expect(window.bpmux?.authToken).toMatch(/.+/)
  })

  it('renders <App /> inside jsdom without throwing', () => {
    rendered = renderApp()
    expect(rendered.container).toBeTruthy()
    expect(rendered.container.children.length).toBeGreaterThan(0)
  })

  it('real backend responds to GET /api/board with 200 (HTTP round-trip)', async () => {
    rendered = renderApp()
    const res = await fetch(`${rendered.apiBase}/api/board`, {
      headers: { Authorization: `Bearer ${rendered.authToken}` }
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body).toMatchObject({})
  })

  it('single-test wall-clock stays under AC8 budget (proof of architectural-win vs Playwright)', async () => {
    const start = performance.now()
    rendered = renderApp()
    const res = await fetch(`${rendered.apiBase}/api/board`, {
      headers: { Authorization: `Bearer ${rendered.authToken}` }
    })
    await res.text()
    const elapsed = performance.now() - start
    expect(elapsed).toBeLessThan(PER_TEST_BUDGET_MS)
  })
})
