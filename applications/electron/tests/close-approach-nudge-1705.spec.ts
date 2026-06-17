import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import type { Page } from '@playwright/test'

const NOT_READY = { drafted: true, stories_ordered: true, review_closed: false, dev_ok: false }
const READY = { drafted: true, stories_ordered: true, review_closed: true, dev_ok: true }

const member = (position: number, state: string): Record<string, unknown> => ({
  number: 500 + position,
  title: `member ${position}`,
  state,
  labels: ['Feature'],
  assignees: [],
  created_at: '2026-06-16T00:00:00Z',
  body: '',
  iteration_membership: { iteration_number: 100, position }
})

const boardWith = (nextReadiness: typeof NOT_READY): Record<string, unknown> => ({
  fetched_at: '2026-06-16T00:00:00Z',
  stories: [
    {
      number: 100,
      title: 'Iteration Lupus',
      state: 'in-progress',
      gh_state: 'open',
      labels: ['iteration'],
      assignees: [],
      created_at: '2026-06-16T00:00:00Z',
      body: '',
      is_current: true,
      iteration_order: 1
    },
    {
      number: 200,
      title: 'Iteration Fornax',
      state: 'icebox',
      gh_state: 'open',
      labels: ['iteration'],
      assignees: [],
      created_at: '2026-06-16T00:00:00Z',
      body: '',
      iteration_order: 2,
      readiness: nextReadiness
    },
    member(1, 'delivered'),
    member(2, 'delivered'),
    member(3, 'delivered'),
    member(4, 'in-progress')
  ]
})

const launchWithBoard = async (
  board: Record<string, unknown>,
  reducedMotion?: 'reduce'
): Promise<{ app: Awaited<ReturnType<typeof launchIsolatedElectron>>['app']; win: Page }> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-1705-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-1705-udd-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  if (reducedMotion) await win.emulateMedia({ reducedMotion })
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(board)
    })
  })
  await win.reload()
  return { app, win }
}

type MotionTrace = { minOpacity: number | null; maxAnims: number; samples: number }

const traceFromFirstAppearance = (win: Page): Promise<MotionTrace> =>
  win.evaluate(
    () =>
      new Promise<MotionTrace>((resolve) => {
        const opacities: number[] = []
        const anims: number[] = []
        let postFrames = 0
        let totalFrames = 0
        const tick = (): void => {
          const el = document.querySelector('[data-test="close-approach-nudge"]')
          if (el) {
            opacities.push(parseFloat(getComputedStyle(el).opacity))
            anims.push(el.getAnimations().length)
            postFrames += 1
          }
          totalFrames += 1
          const waitingForAppearance = opacities.length === 0 && totalFrames < 240
          const samplingAfterAppearance = opacities.length > 0 && postFrames < 30
          if (waitingForAppearance || samplingAfterAppearance) requestAnimationFrame(tick)
          else
            resolve({
              minOpacity: opacities.length ? Math.min(...opacities) : null,
              maxAnims: anims.length ? Math.max(...anims) : 0,
              samples: opacities.length
            })
        }
        requestAnimationFrame(tick)
      })
  )

test.describe('#1705 close-approach readiness nudge', () => {
  test('close-approach + next-not-ready: the strip renders in-frame (owner-visible, warn-not-block) + ◐ tab-badge, and the ENTER slide PLAYS (opacity 0→1, getAnimations live)', async () => {
    const { app, win } = await launchWithBoard(boardWith(NOT_READY))
    const trace = traceFromFirstAppearance(win)
    try {
      const strip = win.locator('[data-test="close-approach-nudge"]')
      await expect(strip).toBeVisible()
      await expect(strip).toContainText('Fornax')
      await expect(strip).toContainText('designer-review')
      await expect(win.locator('[data-test="board-roadmap-attn"]')).toBeVisible()
      await expect(win.locator('[data-test="board-list"]')).toBeVisible()

      const t = await trace
      expect(t.samples, 'strip was observed from its first frame').toBeGreaterThan(0)
      expect(t.maxAnims, 'a CSS animation/transition ran on the strip (it plays)').toBeGreaterThan(
        0
      )
      expect(
        t.minOpacity,
        'opacity rose from <1 — the slide-fade entered, not popped'
      ).toBeLessThan(1)
    } finally {
      await app.close()
    }
  })

  test('dismiss collapses the strip (the EXIT plays) but the ◐ tab-badge survives (recognition-over-recall)', async () => {
    const { app, win } = await launchWithBoard(boardWith(NOT_READY))
    try {
      await expect(win.locator('[data-test="close-approach-nudge"]')).toBeVisible()
      await win.locator('[data-test="close-approach-nudge-dismiss"]').click()
      const exitAnims = await win.evaluate(
        () =>
          new Promise<number>((resolve) => {
            let max = 0
            let frames = 0
            const tick = (): void => {
              const el = document.querySelector('[data-test="close-approach-nudge"]')
              if (el) max = Math.max(max, el.getAnimations().length)
              frames += 1
              if (frames < 30) requestAnimationFrame(tick)
              else resolve(max)
            }
            requestAnimationFrame(tick)
          })
      )
      expect(exitAnims, 'the collapse transition ran (exit plays)').toBeGreaterThan(0)
      await expect(win.locator('[data-test="close-approach-nudge"]')).toHaveAttribute(
        'data-dismissed',
        'true'
      )
      await expect(win.locator('[data-test="board-roadmap-attn"]')).toBeVisible()
    } finally {
      await app.close()
    }
  })

  test('next-ready: no strip and no tab-badge (signal-honesty — only an un-ready NEXT trips it)', async () => {
    const { app, win } = await launchWithBoard(boardWith(READY))
    try {
      await expect(win.locator('[data-test="board-list"]')).toBeVisible()
      await expect(win.locator('[data-test="close-approach-nudge"]')).toHaveCount(0)
      await expect(win.locator('[data-test="board-roadmap-attn"]')).toHaveCount(0)
    } finally {
      await app.close()
    }
  })

  test('reduced-motion: the strip is present but does NOT animate (instant, no slide)', async () => {
    const { app, win } = await launchWithBoard(boardWith(NOT_READY), 'reduce')
    const trace = traceFromFirstAppearance(win)
    try {
      await expect(win.locator('[data-test="close-approach-nudge"]')).toBeVisible()
      const t = await trace
      expect(t.maxAnims, 'reduced-motion ran NO animation (instant present)').toBe(0)
      const duration = await win.evaluate(
        () =>
          getComputedStyle(document.querySelector('[data-test="close-approach-nudge"]') as Element)
            .transitionDuration
      )
      expect(duration).toMatch(/^0s|^0ms|0s,/)
    } finally {
      await app.close()
    }
  })
})
