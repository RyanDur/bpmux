import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { TerminalPage } from './pages/terminalPage'

test.describe('terminals re-attach across a window reload — the renderer re-presents the SAME terminal_id and its custom name persists (no respawn, no vanish)', () => {
  test('after a reload the one terminal re-appears under the SAME terminal_id and keeps its renamed label', async () => {
    test.setTimeout(60_000)
    const { app, win, mockGh } = await launchWithMockGh({
      extraEnv: { BPMUX_ROOM_AGENT: '/bin/sh' }
    })
    try {
      const pt = new TerminalPage(win)
      await pt.spawn()
      await pt.waitReady()
      await expect(pt.item(1)).toHaveText(/^1 · \S/)
      const idBeforeReload = await pt.activeTerminalId()

      await pt.rename(1, 'buildbox')
      await expect(pt.item(1)).toHaveText('1 · buildbox')

      await pt.reload()

      await expect(pt.item(1)).toBeVisible({ timeout: 20000 })
      await expect(pt.items()).toHaveCount(1)
      await expect(pt.item(1)).toHaveText('1 · buildbox')

      await pt.waitReady()
      await pt.expectActiveTerminalId(idBeforeReload)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
