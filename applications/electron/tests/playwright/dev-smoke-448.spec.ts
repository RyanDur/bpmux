import { expect, test } from '@playwright/test'
import { launchIsolatedWithWindow } from '../helpers/launch'
import { seedCurrentIterationViaTestApi } from '../helpers/iteration-fixture'
import { BoardPage } from '../pages/boardPage'

test.describe('#1160 — board-group <summary> keyboard activation', () => {
  test('Enter on focused board-group summary toggles aria-expanded + details[open]', async () => {
    const { app, win, backendPort, authToken } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      await seedCurrentIterationViaTestApi(backendPort, authToken, 'Pw448Keyboard')
      await win.reload()
      const board = new BoardPage(win)
      const summary = board.columnToggle('Ready')
      const details = board.group('Ready')
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
      const board = new BoardPage(win)
      const summary = board.columnToggle('Ready')
      const details = board.group('Ready')
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
