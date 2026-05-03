import { expect, test } from '@playwright/test'
import { launchIsolatedWithWindow } from '../helpers/launch'

test.describe('#1160 — board-group <summary> keyboard activation', () => {
  test('Enter on focused board-group summary toggles aria-expanded + details[open]', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    try {
      const summary = win.locator('[data-test="board-group-ready"]')
      const details = win.locator('details.board-group').first()
      await summary.focus()
      await expect(summary).toHaveAttribute('aria-expanded', 'true')
      await expect(details).toHaveJSProperty('open', true)
      await win.keyboard.press('Enter')
      await expect(summary).toHaveAttribute('aria-expanded', 'false')
      await expect(details).toHaveJSProperty('open', false)
      await win.keyboard.press('Space')
      await expect(summary).toHaveAttribute('aria-expanded', 'true')
      await expect(details).toHaveJSProperty('open', true)
    } finally {
      await app.close()
    }
  })

  test('mouse-click toggle preserved (regression guard)', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    try {
      const summary = win.locator('[data-test="board-group-ready"]')
      const details = win.locator('details.board-group').first()
      await expect(details).toHaveJSProperty('open', true)
      await summary.click()
      await expect(details).toHaveJSProperty('open', false)
      await summary.click()
      await expect(details).toHaveJSProperty('open', true)
    } finally {
      await app.close()
    }
  })
})
