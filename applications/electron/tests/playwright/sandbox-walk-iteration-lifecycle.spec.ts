import { expect, test } from '@playwright/test'
import { mkdirSync, existsSync } from 'fs'
import { join } from 'path'
import { launchIsolatedWithWindow } from '../helpers/launch'
import type { Page } from '@playwright/test'
import { BoardPage } from '../pages/boardPage'

type BoardTab = 'Now' | 'Plan' | 'Done'

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

const switchToTab = async (win: Page, label: BoardTab): Promise<void> => {
  const board = new BoardPage(win)
  await board.openTab(label)
  await expect(board.tab(label)).toBeChecked({ timeout: 5_000 })
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

const parseCreatedIterationNumber = (msg: unknown): number => {
  const match = String(msg ?? '').match(/iteration created: #(\d+) /)
  expect(match, 'create_iteration must return numbered confirmation').not.toBeNull()
  return Number(match![1])
}

const parseCreatedStoryNumber = (msg: unknown): number => {
  const match = String(msg ?? '').match(/#(\d+)/)
  expect(match, 'create_story must return a numbered confirmation').not.toBeNull()
  return Number(match![1])
}

const unscheduledOrder = (win: Page): Promise<number[]> => new BoardPage(win).unscheduledOrder()

test.describe('#1569 — live-walk harness slice 2/2: full iteration lifecycle drive+capture', () => {
  test.setTimeout(90_000)
  test('designate → reorder → rename → close, each step capturable + visually featured', async () => {
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      const token = await win.evaluate(
        () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
      )
      const stamp = Date.now()
      const nameA = `LifecycleWalkA-${stamp}`
      const nameB = `LifecycleWalkB-${stamp}`
      const nameC = `LifecycleWalkC-${stamp}`

      const createA = await postTest(backendPort, token, 'create_iteration', {
        name: nameA,
        agent: 'pm:test'
      })
      expect(createA.ok, JSON.stringify(createA.json)).toBe(true)
      const numberA = parseCreatedIterationNumber(createA.json.message)

      const createB = await postTest(backendPort, token, 'create_iteration', {
        name: nameB,
        agent: 'pm:test'
      })
      expect(createB.ok, JSON.stringify(createB.json)).toBe(true)
      parseCreatedIterationNumber(createB.json.message)

      const createC = await postTest(backendPort, token, 'create_iteration', {
        name: nameC,
        agent: 'pm:test'
      })
      expect(createC.ok, JSON.stringify(createC.json)).toBe(true)
      parseCreatedIterationNumber(createC.json.message)

      await switchToTab(win, 'Plan')
      await expect(win.getByText(`Iteration ${nameA}`).first()).toBeVisible({
        timeout: 15_000
      })
      await capture(win, 'lifecycle-01-created')

      const designate = await postTest(backendPort, token, 'designate_current_iteration', {
        iteration_number: numberA,
        agent: 'pm:test'
      })
      expect(designate.ok, JSON.stringify(designate.json)).toBe(true)

      await switchToTab(win, 'Now')
      const headerIteration = new BoardPage(win).headerIteration()
      await expect(headerIteration).toBeVisible({ timeout: 15_000 })
      await expect(headerIteration).toContainText(nameA, {
        timeout: 15_000
      })
      await capture(win, 'lifecycle-02-designated')

      const createX = await postTest(backendPort, token, 'create_story', {
        title: `ReorderWalkX-${stamp}`,
        user_job: 'reorder-step backlog item X',
        observation_falsifier: 'reject if X cannot be reordered',
        agent: 'pm:test'
      })
      expect(createX.ok, JSON.stringify(createX.json)).toBe(true)
      const numberX = parseCreatedStoryNumber(createX.json.message)

      const createY = await postTest(backendPort, token, 'create_story', {
        title: `ReorderWalkY-${stamp}`,
        user_job: 'reorder-step backlog item Y',
        observation_falsifier: 'reject if Y cannot be reordered',
        agent: 'pm:test'
      })
      expect(createY.ok, JSON.stringify(createY.json)).toBe(true)
      const numberY = parseCreatedStoryNumber(createY.json.message)

      await switchToTab(win, 'Plan')
      await expect
        .poll(() => unscheduledOrder(win), { timeout: 15_000 })
        .toEqual(expect.arrayContaining([numberX, numberY]))
      const orderBefore = await unscheduledOrder(win)
      expect(orderBefore.indexOf(numberX)).toBeLessThan(orderBefore.indexOf(numberY))

      const reorder = await postTest(backendPort, token, 'move_story', {
        number: numberY,
        target_state: 'icebox',
        position: 1,
        rationale: 'live-walk slice 2/2 reorder-step — promote Y above X in the backlog',
        agent: 'pm:test'
      })
      expect(reorder.ok, JSON.stringify(reorder.json)).toBe(true)

      await switchToTab(win, 'Plan')
      await expect
        .poll(
          async () => {
            const order = await unscheduledOrder(win)
            return order.indexOf(numberY) < order.indexOf(numberX)
          },
          { timeout: 15_000 }
        )
        .toBe(true)
      await capture(win, 'lifecycle-03-reordered')

      const renameTitle = `Iteration ${nameA}-RENAMED`
      const rename = await postTest(backendPort, token, 'edit_story', {
        number: numberA,
        title: renameTitle,
        agent: 'pm:test'
      })
      expect(rename.ok, JSON.stringify(rename.json)).toBe(true)
      await switchToTab(win, 'Now')
      await expect(headerIteration).toBeVisible({ timeout: 30_000 })
      await expect(headerIteration).toContainText(`${nameA}-RENAMED`, {
        timeout: 15_000
      })
      await capture(win, 'lifecycle-04-renamed')

      const close = await postTest(backendPort, token, 'close_story', {
        number: numberA,
        reason: 'live-walk slice 2/2 close-step substrate-verification',
        agent: 'pm:test'
      })
      expect(close.ok, JSON.stringify(close.json)).toBe(true)
      await switchToTab(win, 'Now')
      await expect(headerIteration).toBeHidden({ timeout: 15_000 })
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
