import { expect, test } from '@playwright/test'
import { mkdirSync, existsSync } from 'fs'
import { join } from 'path'
import { launchIsolatedWithWindow } from '../helpers/launch'
import type { Page } from '@playwright/test'

const CAPTURE_DIR = join(__dirname, '__captures__')

const ensureCaptureDir = (): void => {
  if (!existsSync(CAPTURE_DIR)) mkdirSync(CAPTURE_DIR, { recursive: true })
}

const capture = async (win: Page, name: string): Promise<string> => {
  ensureCaptureDir()
  const path = join(CAPTURE_DIR, `${name}.png`)
  await win.screenshot({ path, fullPage: true })
  return path
}

const switchToTab = async (win: Page, tabId: string): Promise<void> => {
  const tab = win.locator(`[data-test="board-tab-${tabId}"]`).first()
  const fallback = win.locator(`role=tab[name=/${tabId}/i]`).first()
  if (await tab.isVisible().catch(() => false)) {
    await tab.click()
    return
  }
  if (await fallback.isVisible().catch(() => false)) {
    await fallback.click()
  }
}

const postTest = async (
  port: number,
  token: string,
  path: string,
  body: Record<string, unknown>
): Promise<{ ok: boolean; status: number; json: Record<string, unknown> }> => {
  const res = await fetch(`http://127.0.0.1:${port}/api/test/${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
    body: JSON.stringify(body)
  })
  const json = (await res.json()) as Record<string, unknown>
  return { ok: res.ok, status: res.status, json }
}

test.describe('#1569 — live-walk harness slice 2/2: full iteration lifecycle drive+capture', () => {
  test('designate → reorder → rename → close, each step capturable, isolated from production', async () => {
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      const token = await win.evaluate(
        () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
      )

      const theme = `LifecycleWalk-${Date.now()}`
      const createA = await postTest(backendPort, token, 'create_iteration', {
        theme_name: theme,
        agent: 'pm:test'
      })
      expect(createA.ok, JSON.stringify(createA.json)).toBe(true)
      const createMatch = String(createA.json.message ?? '').match(/iteration created: #(\d+) /)
      expect(createMatch, 'create_iteration must return numbered confirmation').not.toBeNull()
      const iterationNumber = Number(createMatch![1])

      await expect(win.locator(`text=Iteration ${theme}`).first()).toBeAttached({ timeout: 15_000 })
      await switchToTab(win, 'plan')
      await capture(win, 'lifecycle-01-created')

      const designate = await postTest(backendPort, token, 'designate_current_iteration', {
        iteration_number: iterationNumber,
        agent: 'pm:test'
      })
      expect(designate.ok, JSON.stringify(designate.json)).toBe(true)

      await switchToTab(win, 'now')
      await expect(win.locator(`text=Iteration ${theme}`).first()).toBeAttached({ timeout: 15_000 })
      await capture(win, 'lifecycle-02-designated')

      const reorderTheme = `LifecycleWalk-Sister-${Date.now()}`
      const createB = await postTest(backendPort, token, 'create_iteration', {
        theme_name: reorderTheme,
        agent: 'pm:test'
      })
      expect(createB.ok, JSON.stringify(createB.json)).toBe(true)
      const reorderMatch = String(createB.json.message ?? '').match(/iteration created: #(\d+) /)
      expect(reorderMatch, 'create_iteration B must return numbered confirmation').not.toBeNull()
      const reorderNumber = Number(reorderMatch![1])
      const reorder = await postTest(backendPort, token, 'move_story', {
        number: reorderNumber,
        target_state: 'icebox',
        position: 1,
        rationale: 'live-walk slice 2/2 reorder-step substrate-verification',
        agent: 'pm:test'
      })
      expect(reorder.ok, JSON.stringify(reorder.json)).toBe(true)
      await switchToTab(win, 'plan')
      await expect(win.locator(`text=Iteration ${reorderTheme}`).first()).toBeAttached({
        timeout: 15_000
      })
      await capture(win, 'lifecycle-03-reordered')

      const renameTitle = `Iteration ${theme}-RENAMED`
      const rename = await postTest(backendPort, token, 'edit_story', {
        number: iterationNumber,
        title: renameTitle,
        agent: 'pm:test'
      })
      expect(rename.ok, JSON.stringify(rename.json)).toBe(true)
      await switchToTab(win, 'now')
      await expect(win.locator(`text=${renameTitle}`).first()).toBeAttached({ timeout: 15_000 })
      await capture(win, 'lifecycle-04-renamed')

      const close = await postTest(backendPort, token, 'close_story', {
        number: iterationNumber,
        reason: 'live-walk slice 2/2 close-step substrate-verification',
        agent: 'pm:test'
      })
      expect(close.ok, JSON.stringify(close.json)).toBe(true)
      await capture(win, 'lifecycle-05-closed')

      expect(backendPort).not.toBe(4117)
      const apiBase = await win.evaluate(
        () => (window as unknown as { bpmux?: { apiBase?: string } }).bpmux?.apiBase ?? ''
      )
      expect(
        apiBase,
        'sandbox window must point at its ephemeral port, not owner port 4117'
      ).toContain(`:${backendPort}`)
    } finally {
      await app.close()
    }
  })
})
