import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

test.describe('#467 board panel — bounded scroll + sticky headers + native <details>', () => {
  test('section toggling uses native <details>/<summary> primitives', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const detailsCount = await win.locator('details.board-group').count()
      expect(detailsCount).toBeGreaterThan(0)

      const summaryCount = await win
        .locator('details.board-group > summary.board-group-summary')
        .count()
      expect(summaryCount).toBe(detailsCount)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('default browser disclosure-triangle is suppressed', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const summary = win.locator('[data-test="board-group-ready"]')
      const listStyle = await summary.evaluate((el) => getComputedStyle(el).listStyleType)
      expect(listStyle).toBe('none')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('multi-expand: opening one section does not auto-collapse others', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const ready = win.locator('[data-test="board-group-ready"]')
      const inProgress = win.locator('[data-test="board-group-in-progress"]')
      const accepted = win.locator('[data-test="board-group-accepted"]')

      await expect(ready).toHaveAttribute('aria-expanded', 'true')
      await expect(inProgress).toHaveAttribute('aria-expanded', 'true')

      await win.locator('[data-test="board-tab-done"]').click()
      await expect(accepted).toHaveAttribute('aria-expanded', 'false')

      await accepted.click()
      await expect(accepted).toHaveAttribute('aria-expanded', 'true')

      await win.locator('[data-test="board-tab-now"]').click()
      await expect(ready).toHaveAttribute('aria-expanded', 'true')
      await expect(inProgress).toHaveAttribute('aria-expanded', 'true')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('board-body has bounded vertical scroll affordance', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const body = win.locator('[data-test="board-list"]')
      const overflowY = await body.evaluate((el) => getComputedStyle(el).overflowY)
      expect(overflowY).toBe('auto')

      const minHeight = await body.evaluate((el) => getComputedStyle(el).minHeight)
      expect(minHeight).toBe('0px')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('section headers are sticky-positioned with opaque background', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const summary = win.locator('[data-test="board-group-ready"]')
      const position = await summary.evaluate((el) => getComputedStyle(el).position)
      expect(position).toBe('sticky')

      const top = await summary.evaluate((el) => getComputedStyle(el).top)
      expect(top).toBe('0px')

      const bg = await summary.evaluate((el) => getComputedStyle(el).backgroundColor)
      expect(bg).not.toBe('rgba(0, 0, 0, 0)')
      expect(bg).not.toBe('transparent')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('chevron rotates between collapsed and expanded via [open] selector', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const summary = win.locator('[data-test="board-group-ready"]')
      const chevron = summary.locator('.board-group-chevron')
      const before = (): Promise<string> =>
        chevron.evaluate((el) => getComputedStyle(el, '::before').content)

      expect(await before()).toBe('"▾"')
      await summary.click()
      expect(await before()).toBe('"▸"')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('no console errors or hydration warnings on expand/collapse cycle', async () => {
    const { app, win, mockGh } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const summary = win.locator('[data-test="board-group-ready"]')
      await summary.click()
      await summary.click()
      await summary.click()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
