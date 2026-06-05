import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const MOCK_BOARD = {
  fetched_at: '2026-06-04T00:00:00Z',
  stories: [
    {
      number: 910,
      title: 'Iteration AlphaTheme',
      state: 'in-progress',
      gh_state: 'open',
      labels: ['iteration', 'iteration-order:1'],
      assignees: [],
      created_at: '2026-06-04T00:00:00Z',
      body: '',
      is_current: true
    },
    {
      number: 911,
      title: 'Member of AlphaTheme',
      state: 'in-progress',
      labels: ['Feature'],
      assignees: [],
      created_at: '2026-06-04T00:00:00Z',
      body: '',
      iteration_membership: { iteration_number: 910, position: 1 }
    }
  ]
}

test.describe('#1510 board-header iteration-context', () => {
  test('iteration-name leads the BOARD-header (left, primary) with BOARD + sync-glyph trailing; anchor-box retired', async () => {
    const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-1510-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-1510-udd-'))
    const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
    const win = await app.firstWindow()
    try {
      await win.route('**/api/board', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(MOCK_BOARD)
        })
      })

      await win.evaluate(() => window.localStorage.setItem('bpmux-theme', 'dark'))
      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.theme))
        .toBe('dark')

      const header = win.locator('.board-header')
      const iteration = win.locator('[data-test="board-header-iteration"]')
      const title = header.locator('.board-title')
      const glyph = header.locator('[data-test="sync-status-glyph"]')

      await expect(iteration).toBeVisible()
      await expect(iteration).toHaveText('AlphaTheme')
      await expect(title).toHaveText('Board')
      await expect(glyph).toBeVisible()

      const iterBox = await iteration.boundingBox()
      const titleBox = await title.boundingBox()
      const glyphBox = await glyph.boundingBox()
      expect(iterBox).not.toBeNull()
      expect(titleBox).not.toBeNull()
      expect(glyphBox).not.toBeNull()
      expect(
        iterBox!.x,
        'iteration-name leads on the LEFT, before the BOARD panel-marker'
      ).toBeLessThan(titleBox!.x)
      expect(
        titleBox!.x,
        'BOARD label sits left of the trailing sync-glyph (BOARD ● grouped on the right)'
      ).toBeLessThan(glyphBox!.x)

      const iterColor = await iteration.evaluate((el) => getComputedStyle(el).color)
      const primary = await win.evaluate(() => {
        const probe = document.createElement('span')
        probe.style.color = 'var(--fg-primary)'
        document.body.appendChild(probe)
        const c = getComputedStyle(probe).color
        probe.remove()
        return c
      })
      expect(iterColor, 'iteration-name carries --fg-primary (the hero weight)').toBe(primary)

      await win.locator('[data-test="board-tab-now"]').click()
      await expect(win.locator('[data-test="board-tabpanel-now-content"]')).toBeVisible()
      await expect(win.locator('[data-test="board-now-iteration-anchor"]')).toHaveCount(0)

      const shotPath = join(tmpdir(), 'bpmux-1510-board-header.png')
      await win.screenshot({ path: shotPath, fullPage: false })
    } finally {
      await app.close()
    }
  })
})
