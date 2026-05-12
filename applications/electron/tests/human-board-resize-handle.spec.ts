import { test, expect, type Page } from '@playwright/test'
import { launchIsolatedWithWindow, launchIsolatedElectron } from './helpers/launch'

const flexBasisPx = (win: Page): Promise<number> =>
  win
    .locator('[data-test="board-region"]')
    .evaluate((el) => Number.parseInt(getComputedStyle(el).flexBasis, 10))

const waitForFlexBasis = async (win: Page, expected: number): Promise<void> => {
  await expect.poll(() => flexBasisPx(win), { timeout: 5000 }).toBe(expected)
}

test.describe('#458 board panel — drag-resize + persistence', () => {
  test('handle is visible with col-resize cursor on hover', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const handle = win.locator('[data-test="board-resize-handle"]')
    await expect(handle).toBeVisible()
    const cursor = await handle.evaluate((el) => getComputedStyle(el).cursor)
    expect(cursor).toBe('col-resize')
    await app.close()
  })

  test('handle exposes WAI-ARIA windowsplitter contract', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const handle = win.locator('[data-test="board-resize-handle"]')
    await expect(handle).toHaveAttribute('role', 'separator')
    await expect(handle).toHaveAttribute('aria-orientation', 'vertical')
    await expect(handle).toHaveAttribute('aria-controls', 'board-region')
    await expect(handle).toHaveAttribute('aria-label', 'Board panel width')
    await expect(handle).toHaveAttribute('aria-valuemin', '0')
    await expect(handle).toHaveAttribute('aria-valuemax', '100')
    const valuenow = await handle.getAttribute('aria-valuenow')
    expect(valuenow).not.toBeNull()
    expect(Number.parseInt(valuenow!, 10)).toBeGreaterThanOrEqual(0)
    expect(Number.parseInt(valuenow!, 10)).toBeLessThanOrEqual(100)
    await app.close()
  })

  test('default width on fresh userDataDir is 360px', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const board = win.locator('[data-test="board-region"]')
    const flexBasis = await board.evaluate((el) => getComputedStyle(el).flexBasis)
    expect(flexBasis).toBe('360px')
    await app.close()
  })

  test('Left arrow shrinks, Right arrow grows, aria-valuenow updates (WAI-ARIA APG)', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const handle = win.locator('[data-test="board-resize-handle"]')

    await handle.focus()
    const beforeAriaNow = await handle.getAttribute('aria-valuenow')

    await win.keyboard.press('ArrowLeft')
    await waitForFlexBasis(win, 336)
    const afterShrinkAriaNow = await handle.getAttribute('aria-valuenow')
    expect(afterShrinkAriaNow).not.toBe(beforeAriaNow)
    expect(Number.parseInt(afterShrinkAriaNow!, 10)).toBeLessThan(
      Number.parseInt(beforeAriaNow!, 10)
    )

    await win.keyboard.press('ArrowRight')
    await waitForFlexBasis(win, 360)

    await app.close()
  })

  test('keyboard cannot push width below min (260px)', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const handle = win.locator('[data-test="board-resize-handle"]')
    await handle.focus()
    for (let i = 0; i < 100; i++) await win.keyboard.press('ArrowLeft')
    await waitForFlexBasis(win, 260)
    await app.close()
  })

  test('Home jumps to min width, End jumps to max', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const handle = win.locator('[data-test="board-resize-handle"]')
    await handle.focus()

    await win.keyboard.press('Home')
    await waitForFlexBasis(win, 260)

    await win.keyboard.press('End')
    const viewport = await win.evaluate(() => window.innerWidth)
    const expectedMax = Math.min(Math.floor(viewport * 0.5), 800)
    await waitForFlexBasis(win, expectedMax)

    await app.close()
  })

  test('Enter toggles collapse-to-min and restore', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const handle = win.locator('[data-test="board-resize-handle"]')
    await handle.focus()

    await waitForFlexBasis(win, 360)

    await win.keyboard.press('Enter')
    await waitForFlexBasis(win, 260)

    await win.keyboard.press('Enter')
    await waitForFlexBasis(win, 360)

    await app.close()
  })

  test('width persists across app restart per viewer-identity', async () => {
    const first = await launchIsolatedWithWindow({ preserveTempPaths: true })
    const handle = first.win.locator('[data-test="board-resize-handle"]')
    await handle.focus()
    await first.win.keyboard.press('ArrowRight')
    await first.win.keyboard.press('ArrowRight')
    await waitForFlexBasis(first.win, 408)

    const userDataDir = first.userDataDir
    const sessionBase = first.sessionBase
    await first.app.close()

    const second = await launchIsolatedElectron({ userDataDir, sessionBase })
    const win2 = await second.app.firstWindow()
    await expect(win2.locator('[data-test="board-region"]')).toBeVisible()
    await waitForFlexBasis(win2, 408)

    await second.app.close()
  })

  test('rapid drag does not lag main thread (DOM-direct, no React per-move)', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const result = await win.evaluate(async () => {
      const handle = document.querySelector<HTMLElement>('[data-test="board-resize-handle"]')
      const aside = document.getElementById('board-region')
      if (!handle || !aside) return { ok: false, reason: 'no-elements' }

      const rect = handle.getBoundingClientRect()
      const startX = rect.left + rect.width / 2
      const startY = rect.top + rect.height / 2

      handle.dispatchEvent(
        new PointerEvent('pointerdown', {
          pointerId: 1,
          bubbles: true,
          clientX: startX,
          clientY: startY
        })
      )

      const t0 = performance.now()
      for (let i = 1; i <= 60; i++) {
        handle.dispatchEvent(
          new PointerEvent('pointermove', {
            pointerId: 1,
            bubbles: true,
            clientX: startX - i,
            clientY: startY
          })
        )
      }
      const tEnd = performance.now()

      const flexAfterMoves = aside.style.flex

      handle.dispatchEvent(
        new PointerEvent('pointerup', {
          pointerId: 1,
          bubbles: true,
          clientX: startX - 60,
          clientY: startY
        })
      )

      return {
        ok: true,
        elapsedMs: tEnd - t0,
        flexAfterMoves
      }
    })

    expect(result.ok).toBe(true)
    expect(result.elapsedMs).toBeLessThan(100)
    expect(result.flexAfterMoves).toMatch(/^0 0 \d+px$/)

    await app.close()
  })

  test('no console errors during resize cycle', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="board-region"]')).toBeVisible()
    consoleErrors.length = 0
    pageErrors.length = 0

    const handle = win.locator('[data-test="board-resize-handle"]')
    await handle.focus()
    for (const k of ['ArrowLeft', 'ArrowRight', 'Home', 'End', 'Enter', 'Enter']) {
      await win.keyboard.press(k)
    }

    const resizeRelevant = consoleErrors.filter(
      (e) => !/502 \(Bad Gateway\)|Failed to load resource/.test(e)
    )
    expect(resizeRelevant).toEqual([])
    expect(pageErrors).toEqual([])
    await app.close()
  })
})
