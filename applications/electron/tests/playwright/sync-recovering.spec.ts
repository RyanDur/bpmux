import { expect, test } from '@playwright/test'
import { mkdtempSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from '../helpers/launch'

test.describe('#1159 — corrupted board-store surfaces "recovering" indicator state', () => {
  test('corrupted disk file triggers recovering data-state on launch', async () => {
    const storePath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-corrupt-store-')),
      'board-store.json'
    )
    writeFileSync(storePath, '{not valid json at all')

    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: {
        BPMUX_BOARD_STORE_PATH: storePath,
        BPMUX_TEST_MODE: '1',
        BPMUX_TEST_INGEST_FAIL: '1',
        // #1290 Part B: opt out of the default MemoryStorage — this
        // test seeds a corrupted disk file + asserts the renderer
        // surfaces 'recovering' state. MemoryStorage doesn't read
        // disk files at all.
        BPMUX_STORAGE_BACKEND: 'file'
      }
    })
    try {
      const indicator = win.locator('[data-test="sync-status-glyph"]')
      await expect(indicator).toBeVisible()
      await expect(indicator).toHaveAttribute('data-state', 'recovering', { timeout: 15_000 })
      await expect(indicator).toHaveAttribute(
        'aria-label',
        /recovering from local-cache corruption/i
      )
    } finally {
      await app.close()
    }
  })

  test('non-corrupt healthy launch does not show recovering state', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      const indicator = win.locator('[data-test="sync-status-glyph"]')
      await expect(indicator).toBeVisible()
      await expect(indicator).not.toHaveAttribute('data-state', 'recovering')
    } finally {
      await app.close()
    }
  })
})
