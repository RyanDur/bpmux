import { expect, test } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { formatViolations, runA11yAudit } from './helpers/a11yAudit'
import { atpPath } from './helpers/atp'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'
import { MetricsPage } from './pages/metricsPage'
import { SidebarPage } from './pages/sidebarPage'

const WAIVED_RULES: readonly string[] = []

test('every surface holds the ZERO-violation axe floor in both themes — regressions fail here, not in a report nobody reads', async () => {
  test.setTimeout(240_000)
  const { app, win, rpc } = await launchWithMockGh()
  const board = new BoardPage(win)
  const chat = new ChatPage(win)
  const drill = new DrillInPage(win)
  const metrics = new MetricsPage(win)
  const sidebar = new SidebarPage(win)

  const failures: string[] = []
  const audit = async (surface: string): Promise<void> => {
    const result = await runA11yAudit(win)
    const unwaived = result.violations.filter((v) => !WAIVED_RULES.includes(v.id))
    if (unwaived.length > 0) failures.push(`--- ${surface} ---\n${formatViolations(result)}`)
  }

  await expect(chat.appRoot()).toBeVisible()
  await win.evaluate(() => window.localStorage.setItem('bpmux-theme', 'dark'))
  await win.reload()
  await expect.poll(() => win.evaluate(() => document.documentElement.dataset.theme)).toBe('dark')
  await expect(chat.appRoot()).toBeVisible()
  await expect(sidebar.nav()).toBeVisible()
  await audit('boot: chat + sidebar (dark)')

  const createResp = await rpc(
    'create_story',
    {
      title: '[Feature] a11y sweep surface',
      user_job:
        'As a dev, when I sweep every surface, axe judges each one so regressions surface in CI',
      observation_falsifier: 'reject if any audited surface reports an unwaived violation',
      acceptance_test_path: atpPath(__filename)
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  await setupCurrentIterationViaRpc(rpc, 1, 'a11y sweep iteration')

  const surfacesForTheme = async (theme: string): Promise<void> => {
    await expect(board.region()).toBeVisible()
    await board.openTab('Plan')
    await expect(board.storyNavigate(1)).toBeVisible()
    await audit(`board with a story (${theme})`)

    await board.openStory(1)
    await expect(drill.panel()).toBeVisible()
    await audit(`story drill-in (${theme})`)
    await win.keyboard.press('Escape')
    await expect(drill.panel()).toHaveCount(0)

    await win.keyboard.press('Control+Shift+M')
    await expect(metrics.overlay()).toBeVisible()
    await expect
      .poll(() => metrics.dialog().evaluate((el) => getComputedStyle(el).opacity))
      .toBe('1')
    await audit(`metrics overlay (${theme})`)
    await metrics.closeButton().click()
    await expect(metrics.overlay()).toHaveCount(0)
  }

  await win.reload()
  await surfacesForTheme('dark')

  await win.evaluate(() => window.localStorage.setItem('bpmux-theme', 'light'))
  await win.reload()
  await expect.poll(() => win.evaluate(() => document.documentElement.dataset.theme)).toBe('light')
  await audit('boot: chat + sidebar (light)')
  await surfacesForTheme('light')

  expect(failures, failures.join('\n\n')).toEqual([])
  await app.close()
})
