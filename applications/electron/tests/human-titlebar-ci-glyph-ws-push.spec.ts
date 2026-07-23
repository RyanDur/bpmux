import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { startMockGhServer, type MockGhServer } from './helpers/mock_gh'
import { TitlebarPage } from './pages/titlebarPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const SUCCESS_RUNS = {
  total_count: 1,
  workflow_runs: [
    {
      status: 'completed',
      conclusion: 'success',
      head_sha: 'deadbeef',
      display_title: 'green',
      id: 1,
      name: 'ci'
    }
  ]
}

const launchPushFed = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
  mockGh: MockGhServer
}> => {
  const mockGh = await startMockGhServer()
  mockGh.on({
    method: 'GET',
    match: /^\/repos\/acme\/bpmux\/actions\/runs(\?.*)?$/,
    status: 200,
    bodyJson: SUCCESS_RUNS
  })
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-372-'))
  const { app } = await launchIsolatedElectron({
    dataBase,
    userDataDir: udd,
    extraEnv: {
      BPMUX_GH_BASE_URL: mockGh.baseUrl,
      BPMUX_REPO: 'acme/bpmux',
      GH_TOKEN: 'mock-gh-token'
    }
  })
  return { app, mockGh }
}

test.describe('desk-check #372 CI state arrives over the WS push path (no /api/ci fetch)', () => {
  test('glyph reflects the backend conclusion delivered by the ci_watcher broadcast', async () => {
    const { app, mockGh } = await launchPushFed()
    try {
      const win = await app.firstWindow()
      const glyph = new TitlebarPage(win).ciGlyph()
      await expect(glyph).toBeVisible()
      await expect(glyph).toHaveAttribute('aria-label', 'CI success', { timeout: 15_000 })
      await expect(glyph).toHaveClass(/ci-glyph--success/)
      await glyph.screenshot({
        path: join(EVIDENCE_DIR, '95-ci-glyph-push-success.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
