import { expect, test } from '@playwright/test'
import { launchIsolatedWithWindow } from '../helpers/launch'
import { seedCurrentIterationViaTestApi } from '../helpers/iteration-fixture'

test.describe('#1160 — board-group <summary> keyboard activation', () => {
  test('Enter on focused board-group summary toggles aria-expanded + details[open]', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      await seedCurrentIterationViaTestApi(backendPort, authToken, 'Pw448Keyboard')
      await win.reload()
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
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      await seedCurrentIterationViaTestApi(backendPort, authToken, 'Pw448Mouse')
      await win.reload()
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
