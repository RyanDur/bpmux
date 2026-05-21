import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #361 rework + #397 quota_exhausted fold-in: verify the CI glyph
// class + glyph char + aria-label track the backend conclusion
// through all five states (success / failure / in_progress /
// quota_exhausted / unknown). The original user-story falsifier
// — "glyph color never changes" — is gated by confirming each
// conclusion renders its documented class.

const launchWithCiState = async (
  conclusion: string
): Promise<{ app: Awaited<ReturnType<typeof electron.launch>> }> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-361-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-361-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/ci', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        conclusion,
        last_run_id: 123,
        branch: 'main',
        url: 'https://github.com/RyanDur/bpmux/actions/runs/123',
        head_sha: 'abc1234567',
        display_title: `#361 fixture: ${conclusion}`
      })
    })
  })
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-23T03:00:00Z', stories: [] })
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #361 CI glyph color + glyph tracks conclusion', () => {
  const CASES: Array<{
    conclusion: string
    expectedGlyph: string
    expectedClassSuffix: string
  }> = [
    { conclusion: 'success', expectedGlyph: '✓', expectedClassSuffix: 'success' },
    { conclusion: 'failure', expectedGlyph: '✗', expectedClassSuffix: 'failure' },
    { conclusion: 'in_progress', expectedGlyph: '◌', expectedClassSuffix: 'in_progress' },
    { conclusion: 'quota_exhausted', expectedGlyph: '⏸', expectedClassSuffix: 'quota_exhausted' },
    { conclusion: 'unknown', expectedGlyph: '?', expectedClassSuffix: 'unknown' }
  ]

  for (const { conclusion, expectedGlyph, expectedClassSuffix } of CASES) {
    test(`conclusion '${conclusion}' renders as ${expectedGlyph} with matching class`, async () => {
      const { app } = await launchWithCiState(conclusion)
      const win = await app.firstWindow()
      const glyph = win.locator('[data-test="ci-glyph"]')
      await expect(glyph).toBeVisible()
      // data-conclusion mirrors the parsed state; stable selector
      // for future assertions on the conclusion itself.
      await expect(glyph).toHaveAttribute('data-conclusion', conclusion)
      // Class + text carry the visual signal — both must track the
      // conclusion to satisfy the "never changes" falsifier.
      await expect(glyph).toHaveClass(new RegExp(`ci-glyph--${expectedClassSuffix}\\b`))
      await expect(glyph).toContainText(expectedGlyph)
      await app.close()
    })
  }

  test('quota_exhausted aria-label surfaces the reason for SR users', async () => {
    // ⏸ alone is ambiguous; screen-reader users need "CI unavailable
    // — quota exhausted" so they know this is an infra pause and
    // not a code failure.
    const { app } = await launchWithCiState('quota_exhausted')
    const win = await app.firstWindow()
    const glyph = win.locator('[data-test="ci-glyph"]')
    await expect(glyph).toHaveAttribute('aria-label', /quota exhausted/i)
    await app.close()
  })

  test('success state uses success-green color (catches CSS cascade regression)', async () => {
    // #376-style color-assertion gate: the modifier class must
    // actually translate to a distinct computed color, otherwise
    // the class lands but the CSS rule doesn't apply. Catches a
    // future regression where a parent selector overrides ci-glyph
    // color via higher specificity.
    const { app } = await launchWithCiState('success')
    const win = await app.firstWindow()
    const glyph = win.locator('[data-test="ci-glyph"]')
    const color = await glyph.evaluate((el) => getComputedStyle(el).color)
    // Both light + dark mode success greens translate to non-gray;
    // asserting "not the default fg-secondary gray" suffices
    // without hardcoding a specific hex.
    expect(color).not.toBe('rgb(128, 128, 128)')
    expect(color).not.toBe('rgba(0, 0, 0, 0)')
    await app.close()
  })
})
