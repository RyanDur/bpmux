import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('#468 board panel B2 — section-count badges', () => {
  test('badge visible in collapsed section header', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const acceptedSummary = win.locator('[data-test="board-group-accepted"]')
    await expect(acceptedSummary).toHaveAttribute('aria-expanded', 'false')

    const acceptedCount = win.locator('[data-test="board-group-accepted-count"]')
    await expect(acceptedCount).toBeVisible()
    await expect(acceptedCount).toHaveText(/^\(\d+\)$/)

    await app.close()
  })

  test('badge visible in expanded section header', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const readySummary = win.locator('[data-test="board-group-ready"]')
    await expect(readySummary).toHaveAttribute('aria-expanded', 'true')

    const readyCount = win.locator('[data-test="board-group-ready-count"]')
    await expect(readyCount).toBeVisible()
    await expect(readyCount).toHaveText(/^\(\d+\)$/)

    await app.close()
  })

  test('badge format is (N) integer-in-parens', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const states = ['ready', 'in-progress', 'delivered', 'accepted', 'icebox']
    for (const state of states) {
      const count = win.locator(`[data-test="board-group-${state}-count"]`)
      await expect(count).toHaveText(/^\(\d+\)$/)
    }

    await app.close()
  })

  test('count matches the visible story rows in expanded section', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const readyContent = win.locator('#board-group-ready-content')
    const rows = readyContent.locator('[data-test^="board-story-"]')
    const visible = await rows.count()

    const countText = await win.locator('[data-test="board-group-ready-count"]').innerText()
    const n = Number.parseInt(countText.replace(/[()]/g, ''), 10)
    expect(n).toBe(visible)

    await app.close()
  })

  test('empty (count=0) badge carries data-empty and renders in --fg-dim', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const ready = win.locator('[data-test="board-group-ready-count"]')
    await expect(ready).toHaveText('(0)')
    await expect(ready).toHaveAttribute('data-empty', 'true')

    const dimRgb = await win.evaluate(() => {
      const probe = document.createElement('span')
      probe.style.color = 'var(--fg-dim)'
      document.body.appendChild(probe)
      const c = getComputedStyle(probe).color
      probe.remove()
      return c
    })
    const renderedColor = await ready.evaluate((el) => getComputedStyle(el).color)
    expect(renderedColor).toBe(dimRgb)

    await app.close()
  })

  test('clicking the count badge toggles the section (Coyle entire-area-clickable)', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const summary = win.locator('[data-test="board-group-ready"]')
    const count = win.locator('[data-test="board-group-ready-count"]')

    await expect(summary).toHaveAttribute('aria-expanded', 'true')
    await count.click()
    await expect(summary).toHaveAttribute('aria-expanded', 'false')
    await count.click()
    await expect(summary).toHaveAttribute('aria-expanded', 'true')

    await app.close()
  })

  test('count badge sits between chevron and label inside the summary', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const order = await win.evaluate(() => {
      const summary = document.querySelector<HTMLElement>('[data-test="board-group-ready"]')
      if (!summary) return [] as string[]
      return Array.from(summary.children).map((el) => el.className)
    })
    expect(order[0]).toContain('board-group-chevron')
    expect(order[1]).toContain('board-group-count')
    expect(order[2]).toContain('board-group-label')

    await app.close()
  })

  test('no console errors on count render and toggle cycle', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    consoleErrors.length = 0
    pageErrors.length = 0

    const count = win.locator('[data-test="board-group-ready-count"]')
    await count.click()
    await count.click()

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
