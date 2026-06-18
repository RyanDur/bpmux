import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { startMockGhServer, type MockGhServer } from './helpers/mock_gh'

const runsBody = (conclusion: string): Record<string, unknown> => ({
  total_count: 1,
  workflow_runs: [
    {
      status: 'completed',
      conclusion,
      head_sha: 'abc1234def',
      display_title: `#361 fixture: ${conclusion}`,
      id: 123,
      name: 'ci'
    }
  ]
})

const launchWithCiConclusion = async (
  conclusion: string
): Promise<{ app: Awaited<ReturnType<typeof electron.launch>>; mockGh: MockGhServer }> => {
  const mockGh = await startMockGhServer()
  mockGh.on({
    method: 'GET',
    match: /^\/repos\/acme\/bpmux\/actions\/runs(\?.*)?$/,
    status: 200,
    bodyJson: runsBody(conclusion)
  })
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-361-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-361-'))
  const { app } = await launchIsolatedElectron({
    sessionBase,
    userDataDir: udd,
    extraEnv: {
      BPMUX_GH_BASE_URL: mockGh.baseUrl,
      BPMUX_REPO: 'acme/bpmux',
      GH_TOKEN: 'mock-gh-token'
    }
  })
  return { app, mockGh }
}

test.describe('desk-check #361 CI glyph colour + glyph tracks the real backend conclusion', () => {
  const CASES: Array<{
    conclusion: string
    expectedGlyph: string
  }> = [
    { conclusion: 'success', expectedGlyph: '✓' },
    { conclusion: 'failure', expectedGlyph: '✗' }
  ]

  for (const { conclusion, expectedGlyph } of CASES) {
    test(`backend conclusion '${conclusion}' pushes through to glyph ${expectedGlyph}`, async () => {
      const { app, mockGh } = await launchWithCiConclusion(conclusion)
      try {
        const win = await app.firstWindow()
        const glyph = win.locator('[data-test="ci-glyph"]')
        await expect(glyph).toBeVisible()
        await expect(glyph).toHaveAttribute('data-conclusion', conclusion, { timeout: 15_000 })
        await expect(glyph).toHaveClass(new RegExp(`ci-glyph--${conclusion}\\b`))
        await expect(glyph).toContainText(expectedGlyph)
      } finally {
        await app.close()
        await mockGh.close()
      }
    })
  }

  test('success state uses success-green colour (catches CSS cascade regression)', async () => {
    const { app, mockGh } = await launchWithCiConclusion('success')
    try {
      const win = await app.firstWindow()
      const glyph = win.locator('[data-test="ci-glyph"]')
      await expect(glyph).toHaveAttribute('data-conclusion', 'success', { timeout: 15_000 })
      const color = await glyph.evaluate((el) => getComputedStyle(el).color)
      expect(color).not.toBe('rgb(128, 128, 128)')
      expect(color).not.toBe('rgba(0, 0, 0, 0)')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
