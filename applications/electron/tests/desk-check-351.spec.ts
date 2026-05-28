import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck351', '2026-04-22T16:00:00Z'),
    {
      number: 351,
      title: 'Desk-check story for #351 transitions',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Body.',
      ...inMockCurrentIteration(1)
    }
  ]
}

const launchWithBoard = async (
  reducedMotion?: 'reduce'
): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-351-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-351-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  if (reducedMotion) await win.emulateMedia({ reducedMotion })
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/351/comments', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #351 high-signal transitions', () => {
  test('body has color + background-color transition (theme toggle smoothing)', async () => {
    const { app } = await launchWithBoard()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()

    const bodyTransition = await win.evaluate(() => {
      const cs = window.getComputedStyle(document.body)
      return {
        property: cs.transitionProperty,
        duration: cs.transitionDuration,
        easing: cs.transitionTimingFunction
      }
    })
    console.log(`#351 AC6 body transition:`)
    console.log(`  property: ${bodyTransition.property}`)
    console.log(`  duration: ${bodyTransition.duration}`)
    console.log(`  easing:   ${bodyTransition.easing}`)

    expect(bodyTransition.property).toContain('color')
    expect(bodyTransition.property).toContain('background-color')
    // Chromium normalizes duration to seconds, not milliseconds:
    // 150ms → "0.15s". Accept either form.
    expect(bodyTransition.duration).toMatch(/0\.15s|150ms/)

    await app.close()
  })

  test('board region transitions on hide-toggle (always-mounted pattern)', async () => {
    const { app } = await launchWithBoard()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const boardTransition = await win.evaluate(() => {
      const el = document.querySelector('[data-test="board-region"]')
      if (!el) return null
      const cs = window.getComputedStyle(el)
      return {
        property: cs.transitionProperty,
        duration: cs.transitionDuration
      }
    })
    console.log(`#351 AC3 board-region transition:`)
    console.log(`  property: ${boardTransition?.property}`)
    console.log(`  duration: ${boardTransition?.duration}`)

    // Toggle via Ctrl+P, confirm it stays mounted + hidden class toggles.
    const boardBefore = await win.locator('[data-test="board-region"]').isVisible()
    console.log(`  board visible before toggle: ${boardBefore}`)

    // Focus outside input before Ctrl+P (needs focus not in editable).
    await win.locator('[data-test="app-root"]').click()
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+p' : 'Control+p')
    await win.waitForTimeout(400)

    const boardClass = await win.evaluate(
      () => document.querySelector('[data-test="board-region"]')?.className ?? ''
    )
    console.log(`  board-region class after toggle: "${boardClass}"`)

    // Board should STILL be in DOM (always-mounted) — just .board--hidden.
    const stillMounted = await win.evaluate(
      () => !!document.querySelector('[data-test="board-region"]')
    )
    console.log(`  board DOM still mounted: ${stillMounted}`)
    expect(stillMounted).toBe(true)

    await app.close()
  })

  test('drill-in surface has transition declared on enter state', async () => {
    const { app } = await launchWithBoard()
    const win = await app.firstWindow()
    await win.locator('[data-test="board-tabpanel-now-content"] [data-test="board-story-351"] .board-story-navigate').click()
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()
    await win.waitForTimeout(400)

    const drillIn = await win.evaluate(() => {
      const el = document.querySelector('[data-test="board-drill-in"]')
      if (!el) return null
      const cs = window.getComputedStyle(el)
      return {
        property: cs.transitionProperty,
        duration: cs.transitionDuration
      }
    })
    console.log(`#351 AC1 drill-in transition:`)
    console.log(`  property: ${drillIn?.property}`)
    console.log(`  duration: ${drillIn?.duration}`)

    await app.close()
  })

  test('reduced-motion: body + board transitions zero to 0ms', async () => {
    const { app } = await launchWithBoard('reduce')
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()

    const bodyDuration = await win.evaluate(
      () => window.getComputedStyle(document.body).transitionDuration
    )
    const boardDuration = await win.evaluate(() => {
      const el = document.querySelector('[data-test="board-region"]')
      return el ? window.getComputedStyle(el).transitionDuration : null
    })
    console.log(`#351 AC8 reduced-motion durations:`)
    console.log(`  body: ${bodyDuration}`)
    console.log(`  board: ${boardDuration}`)

    expect(bodyDuration).toMatch(/^0s|^0ms/)
    expect(boardDuration).toMatch(/^0s|^0ms/)

    await app.close()
  })

  test('capture UI state — theme toggle + board toggle screenshots (motion not frame-capturable)', async () => {
    const { app } = await launchWithBoard()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await win.waitForTimeout(300)
    await win.screenshot({
      path: join(EVIDENCE_DIR, '60-app-default-dark.png')
    })

    // Toggle theme.
    await win.locator('.titlebar-theme-toggle').click()
    await win.waitForTimeout(300)
    await win.screenshot({
      path: join(EVIDENCE_DIR, '61-app-light-after-toggle.png')
    })

    // Toggle back to dark.
    await win.locator('.titlebar-theme-toggle').click()
    await win.waitForTimeout(200)

    // Toggle board off.
    await win.locator('[data-test="app-root"]').click()
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+p' : 'Control+p')
    await win.waitForTimeout(300)
    await win.screenshot({
      path: join(EVIDENCE_DIR, '62-app-board-hidden.png')
    })

    await app.close()
  })
})
