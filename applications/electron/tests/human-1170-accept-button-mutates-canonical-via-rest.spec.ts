import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'

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

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const acceptanceTestPath =
        'applications/electron/tests/human-1170-accept-button-mutates-canonical-via-rest.spec.ts'
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

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase: 'Picking up the canonical-mutation accept-button regression.',
        falsifier_reframing:
          'reject if accept-button click does not mutate canonical store BoardEvent::StateTransitioned + StoryClosed.'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        number: 1,
        verified_observation:
          'Drove deliver via /api/rpc/deliver_story; staging accept-button click as the gate under test.'
      })
      expect(deliverResp.status).toBe(200)

      await seedTeamVerdicts(rpc, 1, ['designer'])
      const passResp = await rpc(
        'pass_desk_check',
        { number: 1, acceptor: 'owner', would_file_bug: false },
        'designer'
      )
      expect(passResp.status).toBe(200)
      await seedTeamVerdicts(rpc, 1, ['designer', 'dev'])

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()
      const deliveredCard = win.locator(
        '#board-group-delivered-content [data-test="board-story-1"]'
      )
      await expect(deliveredCard).toContainText('human-1170-accept-button-canonical-mutation')
      await deliveredCard.locator('.board-story-button').click()
      await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()

      const acceptBtn = win.locator('[data-test="drill-in-accept"]')
      await expect(acceptBtn).toBeVisible()
      await acceptBtn.click()

      const bugQuestion = win.locator('[data-test="drill-in-bug-question"]')
      await expect(bugQuestion).toBeVisible()
      await win.locator('[data-test="drill-in-bug-question-no"]').click()

      await expect(async () => {
        const boardResp = await win.evaluate(async () => {
          const bridge = (window as unknown as { bpmux?: { authToken?: string; apiBase?: string } })
            .bpmux
          const r = await fetch(`${bridge?.apiBase}/api/board`, {
            headers: { Authorization: `Bearer ${bridge?.authToken ?? ''}` }
          })
          return r.json()
        })
        const story = (boardResp.stories as Array<{ number: number; state: string }>).find(
          (s) => s.number === 1
        )
        expect(story?.state).toBe('accepted')
      }).toPass({ timeout: 10_000 })

      await win.locator('[data-test="board-tab-done"]').click()
      await win.locator('[data-test="board-group-accepted"]').click()
      const acceptedCard = win.locator('#board-group-accepted-content [data-test="board-story-1"]')
      await expect(acceptedCard).toBeVisible()

      for (const otherState of ['ready', 'in-progress', 'desk-check', 'delivered', 'icebox']) {
        const elsewhere = win.locator(
          `#board-group-${otherState}-content [data-test="board-story-1"]`
        )
        await expect(elsewhere).toHaveCount(0)
      }

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
