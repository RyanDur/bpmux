import { expect, test } from '@playwright/test'
import { launchIsolatedElectron } from './helpers/launch'

interface CapturedFrame {
  type?: string
  message_type?: string
  state_snapshot?: unknown
  state_delta?: { domain: string }
}

const installFrameCapture = (): void => {
  ;(window as unknown as { __bpmuxFrames: CapturedFrame[] }).__bpmuxFrames = []
  const OrigWebSocket = window.WebSocket
  ;(window as unknown as { WebSocket: typeof WebSocket }).WebSocket = class
    extends OrigWebSocket
  {
    constructor(url: string | URL, protocols?: string | string[]) {
      super(url, protocols)
      this.addEventListener('message', (event) => {
        try {
          const parsed = JSON.parse(event.data as string) as CapturedFrame
          ;(
            window as unknown as { __bpmuxFrames: CapturedFrame[] }
          ).__bpmuxFrames.push(parsed)
        } catch {}
      })
    }
  } as unknown as typeof WebSocket
}

const readFrames = async (
  win: Awaited<ReturnType<Awaited<ReturnType<typeof launchIsolatedElectron>>['app']['firstWindow']>>
): Promise<CapturedFrame[]> =>
  win.evaluate(
    () =>
      (window as unknown as { __bpmuxFrames: CapturedFrame[] }).__bpmuxFrames ?? []
  )

test.describe('#1213 race-class elimination — WS-only state-flow integration', () => {
  test('AC8 / falsifier-(e): WS-connect delivers StateSnapshot as first non-Welcome frame', async () => {
    const { app } = await launchIsolatedElectron({})
    const win = await app.firstWindow()
    await win.addInitScript(installFrameCapture)
    await win.reload()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await expect
      .poll(async () => {
        const frames = await readFrames(win)
        return frames.some((f) => f.message_type === 'state-snapshot')
      }, { timeout: 5_000 })
      .toBe(true)
    const frames = await readFrames(win)
    const welcomeIdx = frames.findIndex((f) => f.type === 'welcome')
    const snapshotIdx = frames.findIndex((f) => f.message_type === 'state-snapshot')
    expect(welcomeIdx).toBeGreaterThanOrEqual(0)
    expect(snapshotIdx).toBeGreaterThan(welcomeIdx)
    const between = frames.slice(welcomeIdx + 1, snapshotIdx)
    expect(
      between.filter((f) => f.message_type === 'state-delta').length,
      'no state-delta should appear before the connect-snapshot'
    ).toBe(0)
    await app.close()
  })

  test('AC10 / falsifier-(e): late-arriving server reaction-delta applies via WS without REST poll', async () => {
    const { app, backendPort } = await launchIsolatedElectron({})
    const win = await app.firstWindow()
    await win.addInitScript(installFrameCapture)
    await win.reload()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await expect
      .poll(async () => {
        const frames = await readFrames(win)
        return frames.some((f) => f.message_type === 'state-snapshot')
      }, { timeout: 5_000 })
      .toBe(true)
    const token = await win.evaluate(
      () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
    )
    await fetch(`http://127.0.0.1:${backendPort}/api/reactions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`
      },
      body: JSON.stringify({
        channel: 'broadcast',
        message_key: 'race-class-test',
        emoji: '🟢',
        user: 'ryan'
      })
    })
    await expect
      .poll(async () => {
        const frames = await readFrames(win)
        return frames.some(
          (f) => f.message_type === 'state-delta' && f.state_delta?.domain === 'reactions'
        )
      }, { timeout: 5_000 })
      .toBe(true)
    await app.close()
  })

  test('AC11 / falsifier-(e): reconnect re-delivers StateSnapshot per-connection', async () => {
    const { app } = await launchIsolatedElectron({})
    const win = await app.firstWindow()
    await win.addInitScript(installFrameCapture)
    await win.reload()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await expect
      .poll(async () => {
        const frames = await readFrames(win)
        return frames.filter((f) => f.message_type === 'state-snapshot').length >= 1
      }, { timeout: 5_000 })
      .toBe(true)
    await win.reload()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await expect
      .poll(async () => {
        const frames = await readFrames(win)
        return frames.filter((f) => f.message_type === 'state-snapshot').length >= 1
      }, { timeout: 5_000 })
      .toBe(true)
    await app.close()
  })
})
