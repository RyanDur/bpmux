import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { makeBpmuxCli } from './helpers/bpmux-cli'
import { BoardPage } from './pages/boardPage'

test.describe('agent CLI journey — badge-class — a dev-accepted chore renders the dev-accepted badge', () => {
  test('the team drives a Chore to delivered with acceptor dev via the bpmux CLI and the human sees the dev-accepted badge in Acceptance', async () => {
    test.setTimeout(90_000)
    const { app, win, mockGh, backendPort, authToken } = await launchWithMockGh()
    const cli = makeBpmuxCli(backendPort, authToken)
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const storyId = cli.numberFrom(
        await cli.run('dev', [
          'create',
          '--title',
          '[Chore] agent-cli-dev-accepted',
          '--user-job',
          'As dev, when a dev-accepted chore is delivered, the human watching the board needs the dev-accepted badge so the acceptance axis is unambiguous.',
          '--observation-falsifier',
          'reject if a delivered dev-accepted chore does not render the dev-accepted badge'
        ])
      )
      expect(storyId).toBeGreaterThan(0)

      const iterationId = cli.numberFrom(
        await cli.run('pm', ['iteration', 'create', '--name', 'DevAcceptedJourney'])
      )
      expect(iterationId).toBeGreaterThan(0)
      await cli.run('pm', ['iteration', 'designate', String(iterationId)])
      await cli.run('pm', [
        'iteration',
        'add',
        '--iteration',
        String(iterationId),
        '--story',
        String(storyId),
        '--position',
        '1'
      ])

      await cli.run('dev', [
        'start',
        String(storyId),
        '--paraphrase',
        'Picking up the dev-accepted chore to drive it to delivered through the CLI.',
        '--falsifier',
        'reject if the delivered chore does not carry the dev-accepted acceptance badge'
      ])
      await cli.run('dev', [
        'deliver',
        String(storyId),
        '--observation',
        'Drove deliver via the bpmux CLI; passing desk-check with acceptor dev next.'
      ])
      await cli.run('designer', [
        'verdict',
        String(storyId),
        '--verdict',
        'verified',
        '--grounding',
        'cli-journey: designer verdict grounding the dev-accepted desk-check pass'
      ])
      await cli.run('designer', ['pass', String(storyId), '--acceptor', 'dev'])

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openColumn('Acceptance')

      const badge = board.storyStateBadge(storyId)
      await expect(badge).toBeVisible()
      await expect(badge).toHaveClass(/story-badge--dev-accepted\b/)
      await expect(badge).toContainText('awaiting dev')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      cli.cleanup()
      await app.close()
      await mockGh.close()
    }
  })
})
