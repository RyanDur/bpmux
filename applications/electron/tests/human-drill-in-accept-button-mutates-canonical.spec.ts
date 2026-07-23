import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

test.describe('#1170 — UI accept-button mutates canonical store via REST (no silent-fail)', () => {
  test('accept-button click transitions delivered → accepted in canonical AND renderer', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(board.region()).toBeVisible()

      const acceptanceTestPath = atpPath(__filename)
      const createResp = await rpc(
        'create_story',
        {
          title: '[Bug] human-1170-accept-button-canonical-mutation',
          user_job:
            'When the owner clicks the Accept button in the Electron UI, the story should advance from Delivered to Accepted in the canonical store, not just optimistically in the renderer.',
          observation_falsifier:
            'reject if /api/board still shows state=delivered after the accept-button click resolves',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await setupCurrentIterationViaRpc(rpc, 1, 'Human1170AcceptCanonical')

      const startResp = await rpc('start_story', {
        id: 1,
        pickup_paraphrase: 'Picking up the canonical-mutation accept-button regression.',
        falsifier_reframing:
          'reject if accept-button click does not mutate canonical store StoryEvent::StateTransitioned + StoryClosed.'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        id: 1,
        verified_observation:
          'Drove deliver via /api/rpc/deliver_story; staging accept-button click as the gate under test.'
      })
      expect(deliverResp.status).toBe(200)

      await seedTeamVerdicts(rpc, 1, ['designer'])
      const passResp = await rpc(
        'pass_desk_check',
        { id: 1, acceptor: 'owner', would_file_bug: false },
        'designer'
      )
      expect(passResp.status).toBe(200)
      await seedTeamVerdicts(rpc, 1, ['designer', 'dev'])

      await expect(board.storyRow(1)).toContainText('human-1170-accept-button-canonical-mutation', {
        timeout: 10_000
      })
      await board.openStory(1)
      await expect(drillIn.panel()).toBeVisible()

      const acceptBtn = drillIn.acceptButton()
      await expect(acceptBtn).toBeVisible()
      await acceptBtn.click()

      await expect(drillIn.bugQuestionDialog()).toBeVisible()
      await drillIn.bugQuestionNo().click()

      await expect(async () => {
        const boardResp = await win.evaluate(async () => {
          const bridge = (window as unknown as { bpmux?: { authToken?: string; apiBase?: string } })
            .bpmux
          const r = await fetch(`${bridge?.apiBase}/api/board`, {
            headers: { Authorization: `Bearer ${bridge?.authToken ?? ''}` }
          })
          return r.json()
        })
        const story = (boardResp.stories as Array<{ id: number; state: string }>).find(
          (s) => s.id === 1
        )
        expect(story?.state).toBe('accepted')
      }).toPass({ timeout: 10_000 })

      await board.openTab('Done')
      await board.openColumn('Accepted')
      await expect(board.storyRow(1)).toBeVisible()

      await board.openTab('Now')
      await expect(board.storyNavigate(1)).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
