import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'

// #1127 sub-slice 3: drill-in family — reject form gating. Setup
// chain: create→start→deliver→pass_desk_check lands story in
// DELIVERED column where the reject button surfaces.

test.describe('human-side E2E #476 — direct-action — drill-in reject form', () => {
  test('reject button opens textarea + send is gated on non-empty trimmed note', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const acceptanceTestPath =
        'applications/electron/tests/human-drill-in-reject-form-flow.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] reject-form-target',
          user_job:
            'When the owner rejects a delivered story, the form gates send on a non-empty note.',
          observation_falsifier: 'reject if send button enables on empty/whitespace note',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      const startResp = await rpc('start_story', {
        number: 1,
        pickup_paraphrase: 'Driving the chain so reject form is reachable.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        number: 1,
        verified_observation: 'Drove deliver so designer can pass desk-check next.'
      })
      expect(deliverResp.status).toBe(200)

      const passResp = await rpc('pass_desk_check', { number: 1 }, 'designer')
      expect(passResp.status).toBe(200)

      await win.reload()
      await expect(win.locator('[data-test="board-region"]')).toBeVisible()

      const storyRow = win.locator(
        '#board-group-delivered-content [data-test="board-story-1"]'
      )
      await expect(storyRow).toContainText('reject-form-target')

      consoleErrors.length = 0
      pageErrors.length = 0

      await storyRow.locator('.board-story-button').click()
      await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()

      await win.locator('[data-test="drill-in-reject"]').click()

      const noteInput = win.locator('[data-test="drill-in-reject-note"]')
      await expect(noteInput).toBeVisible()
      const sendButton = win.locator('[data-test="drill-in-reject-send"]')
      await expect(sendButton).toBeDisabled()

      await noteInput.fill('needs rework — accessibility regression on the touched surface')
      await expect(sendButton).toBeEnabled()

      await noteInput.fill('   ')
      await expect(sendButton).toBeDisabled()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
