import { expect, test } from '@playwright/test'
import { launchIsolatedElectron } from './helpers/launch'
import { formatViolations, runA11yAudit } from './helpers/a11yAudit'
import { ChatPage } from './pages/chatPage'

test.describe('#425 a11y audit substrate', () => {
  test('axe runs against running Electron surface and returns structured violations', async () => {
    const { app } = await launchIsolatedElectron()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    const result = await runA11yAudit(win)
    console.log(formatViolations(result))

    expect(result).toMatchObject({
      violations: expect.any(Array),
      passes: expect.any(Number),
      incomplete: expect.any(Number)
    })
    expect(result.passes).toBeGreaterThan(0)

    await app.close()
  })

  test('violations expose target + impact + help so designer can cite specific failures', async () => {
    const { app } = await launchIsolatedElectron()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    const result = await runA11yAudit(win)
    for (const v of result.violations) {
      expect(v.id).toEqual(expect.any(String))
      expect(v.help).toEqual(expect.any(String))
      expect(v.helpUrl).toMatch(/^https?:\/\//)
      expect(Array.isArray(v.nodes)).toBe(true)
      for (const n of v.nodes) {
        expect(Array.isArray(n.target)).toBe(true)
      }
    }

    await app.close()
  })
})

const WAIVED_RULES: readonly string[] = []

test('the app holds a ZERO-violation axe floor — a11y regressions fail here, not in a report nobody reads', async () => {
  const { app } = await launchIsolatedElectron()
  const win = await app.firstWindow()
  await expect(new ChatPage(win).appRoot()).toBeVisible()

  const result = await runA11yAudit(win)
  const unwaived = result.violations.filter((v) => !WAIVED_RULES.includes(v.id))
  expect(
    unwaived.map((v) => `${v.id}: ${v.help} (${v.nodes.length} nodes)`),
    formatViolations(result)
  ).toEqual([])

  await app.close()
})
