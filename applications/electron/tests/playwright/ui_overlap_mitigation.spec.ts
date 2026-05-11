// #1286: regression test for the off-screen-positioning mitigation
// (Option 2 of the spec body). Verifies that test-spawned Electron
// windows on bpmux-mac-m1 (self-hosted runner that shares the screen
// with Ryan's live dev work) are positioned off-screen so they don't
// visually interfere with Ryan's screen.
//
// Strategy: Playwright launches Electron via `launchIsolatedElectron`
// which (post-#1286 impl) sets BPMUX_TEST_WINDOW_POSITION=offscreen.
// The Electron main process reads that env var and constructs its
// BrowserWindow at x:-10000, y:-10000 (well outside any reasonable
// monitor). This test verifies the window's actual bounds match the
// off-screen coords.

import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from '../helpers/launch'

test.describe('#1286 — UI-overlap mitigation: off-screen window positioning', () => {
  test('test-spawned Electron window is positioned off-screen (x<-1000)', async () => {
    const { app, win } = await launchIsolatedWithWindow()
    try {
      // Read the window's actual screen bounds via Electron API. The
      // BrowserWindow.getBounds() returns {x, y, width, height} in
      // screen pixels. With #1286 off-screen-positioning active, x
      // should be well-negative (-10000 per impl) so the window is
      // entirely off the primary display.
      const bounds = await app.evaluate(async ({ BrowserWindow }) => {
        const wins = BrowserWindow.getAllWindows()
        if (wins.length === 0) {
          return null
        }
        return wins[0].getBounds()
      })
      expect(bounds, 'BrowserWindow exists').not.toBeNull()
      expect(
        bounds!.x,
        'off-screen x-coord (#1286 Option 2): negative + well below screen origin'
      ).toBeLessThan(-1000)
      // Window still functional — sanity check via DOM accessibility
      // (the original option 1 BPMUX_TEST_MODE hidden-windows broke
      // exactly this class because macOS throttles hidden windows;
      // off-screen windows are technically visible to the OS).
      await expect(win.locator('body')).toBeVisible()
    } finally {
      await app.close()
    }
  })
})
