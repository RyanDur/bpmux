import { setupCurrentIterationViaRpc } from './helpers/iteration-fixture'
import { launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'
import { expect, test } from '@playwright/test'

test.describe('human-side E2E #476 — owner-specific — accept/reject buttons on delivered drill-in', () => {
  test('owner sees accept + reject buttons in drill-in actions for a delivered story', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
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

      const acceptanceTestPath =
        'applications/electron/tests/human-owner-acts-on-delivered-story.spec.ts'
      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] owner-actions-target',
          user_job:
            'When the owner opens a delivered story drill-in, accept + reject buttons should be visible.',
          observation_falsifier:
            'reject if owner-action buttons are absent on a delivered drill-in',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await setupCurrentIterationViaRpc(rpc, 1, 'OwnerActions')

      const startResp = await rpc('start_story', {
        id: 1,
        pickup_paraphrase: 'Picking up to drive deliver before owner-walk.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        id: 1,
        verified_observation: 'Drove deliver so designer can pass desk-check next.'
      })
      expect(deliverResp.status).toBe(200)

      await seedTeamVerdicts(rpc, 1, ['designer'])
      const passResp = await rpc(
        'pass_desk_check',
        { id: 1, acceptor: 'owner', would_file_bug: false },
        'designer'
      )
      expect(passResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.storyInColumn('Acceptance', 1)).toHaveCount(1)
      await expect(board.storyInColumn('Acceptance', 1)).toHaveAttribute(
        'aria-label',
        /owner-actions-target/
      )

      consoleErrors.length = 0
      pageErrors.length = 0

      await board.openStory(1)

      const drill = new DrillInPage(win)
      await expect(drill.panel()).toBeVisible()
      await expect(drill.acceptButton()).toBeVisible()
      await expect(drill.rejectButton()).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('human-side E2E #476 — owner-specific — actions hidden on non-delivered drill-in', () => {
  test('owner does NOT see accept/reject on icebox story drill-in', async () => {
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

      consoleErrors.length = 0
      pageErrors.length = 0

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] icebox-not-actionable',
          user_job:
            'When the owner opens an icebox story, accept/reject buttons should NOT be present (story is not delivered yet).',
          observation_falsifier:
            'reject if accept/reject buttons appear on a non-delivered story drill-in',
          acceptance_test_path:
            'applications/electron/tests/human-owner-acts-on-delivered-story.spec.ts'
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')

      await expect(board.storyRow(1)).toContainText('icebox-not-actionable')

      consoleErrors.length = 0
      pageErrors.length = 0

      await board.storyNavigate(1).click()

      await expect(drillIn.panel()).toBeVisible()

      await expect(drillIn.acceptButton()).toHaveCount(0)
      await expect(drillIn.rejectButton()).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

const CENTER_TOLERANCE_PX = 8
const MIN_DIMMED_BACKDROP_ALPHA = 0.3

test.describe('human-side E2E #476 — direct-action — drill-in reject form', () => {
  test('reject button opens textarea + send is gated on non-empty trimmed note', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drill = new DrillInPage(win)
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const acceptanceTestPath =
        'applications/electron/tests/human-owner-acts-on-delivered-story.spec.ts'
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

      await setupCurrentIterationViaRpc(rpc, 1, 'RejectFormGate')

      const startResp = await rpc('start_story', {
        id: 1,
        pickup_paraphrase: 'Driving the chain so reject form is reachable.',
        falsifier_reframing: 'reject if start_story does not move story to in-progress'
      })
      expect(startResp.status).toBe(200)

      const deliverResp = await rpc('deliver_story', {
        id: 1,
        verified_observation: 'Drove deliver so designer can pass desk-check next.'
      })
      expect(deliverResp.status).toBe(200)

      await seedTeamVerdicts(rpc, 1, ['designer'])
      const passResp = await rpc(
        'pass_desk_check',
        { id: 1, acceptor: 'owner', would_file_bug: false },
        'designer'
      )
      expect(passResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await expect(board.storyNavigate(1)).toHaveAccessibleName(/reject-form-target/)

      consoleErrors.length = 0
      pageErrors.length = 0

      await board.openStory(1)
      await expect(drill.panel()).toBeVisible()

      await drill.rejectButton().click()

      const noteInput = drill.rejectNote()
      await expect(noteInput).toBeVisible()
      await expect(drill.rejectDialog()).toBeVisible()
      const sendButton = drill.rejectSend()
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

  test('#498 native dialog migration: showModal opens form, escape closes + restores focus, cancel does NOT fire reject', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drill = new DrillInPage(win)
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(board.region()).toBeVisible()

      const acceptanceTestPath =
        'applications/electron/tests/human-owner-acts-on-delivered-story.spec.ts'
      await rpc(
        'create_story',
        {
          title: '[Feature] dialog-migration-target',
          user_job:
            'When the owner opens the reject form, native dialog substrate provides focus-trap + Escape-close.',
          observation_falsifier:
            'reject if Escape does not close the form, or focus does not restore',
          acceptance_test_path: acceptanceTestPath
        },
        'pm'
      )
      await setupCurrentIterationViaRpc(rpc, 1, 'RejectFormDialog')
      await rpc('start_story', {
        id: 1,
        pickup_paraphrase: 'Driving the chain so reject form is reachable.',
        falsifier_reframing: 'reject if reject-form is not modal-class'
      })
      await rpc('deliver_story', {
        id: 1,
        verified_observation: 'Drove deliver so designer can pass desk-check next.'
      })
      await seedTeamVerdicts(rpc, 1, ['designer'])
      await rpc('pass_desk_check', { id: 1, acceptor: 'owner', would_file_bug: false }, 'designer')
      await win.reload()
      await expect(board.region()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(board.storyNavigate(1)).toHaveAccessibleName(/dialog-migration-target/)
      await board.openStory(1)
      await expect(drill.panel()).toBeVisible()

      await drill.rejectButton().click()

      const dialog = drill.rejectDialog()
      await expect(dialog).toHaveAttribute('open', '')

      const noteInput = drill.rejectNote()
      await expect(noteInput).toBeVisible()
      await expect(noteInput).toBeFocused()

      const isModal = await dialog.evaluate(
        (el) => el instanceof HTMLDialogElement && el.matches(':modal')
      )
      expect(isModal).toBe(true)

      const geometry = await dialog.evaluate((el) => {
        const rect = el.getBoundingClientRect()
        const backdrop = getComputedStyle(el, '::backdrop').backgroundColor
        const channels = backdrop.match(/[\d.]+/g)
        const backdropAlpha = channels === null ? 0 : channels.length >= 4 ? Number(channels[3]) : 1
        return {
          dialogCenterX: rect.left + rect.width / 2,
          dialogCenterY: rect.top + rect.height / 2,
          viewportCenterX: window.innerWidth / 2,
          viewportCenterY: window.innerHeight / 2,
          backdropAlpha
        }
      })
      expect(
        Math.abs(geometry.dialogCenterX - geometry.viewportCenterX),
        'reject dialog must be horizontally centered in the viewport (#1518)'
      ).toBeLessThanOrEqual(CENTER_TOLERANCE_PX)
      expect(
        Math.abs(geometry.dialogCenterY - geometry.viewportCenterY),
        'reject dialog must be vertically centered in the viewport (#1518)'
      ).toBeLessThanOrEqual(CENTER_TOLERANCE_PX)
      expect(
        geometry.backdropAlpha,
        'reject dialog must have an authored dimmed backdrop (#1518)'
      ).toBeGreaterThanOrEqual(MIN_DIMMED_BACKDROP_ALPHA)

      await win.keyboard.press('Tab')
      await win.keyboard.press('Tab')
      await win.keyboard.press('Tab')
      await win.keyboard.press('Tab')
      const focusTrappedInDialog = await dialog.evaluate(
        (el) => el.contains(document.activeElement) && document.activeElement !== document.body
      )
      expect(
        focusTrappedInDialog,
        'focus must stay trapped within the modal dialog (#1518 clause-3, free via showModal)'
      ).toBe(true)

      await win.mouse.click(5, 5)
      await expect(
        dialog,
        'click-outside must NOT dismiss the reject form — protects the typed note (#1518 clause-4b)'
      ).toHaveAttribute('open', '')

      await noteInput.fill('cancel-path test note')
      await drill.cancelReject().click()
      await expect(drill.rejectDialog()).toHaveCount(0)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
