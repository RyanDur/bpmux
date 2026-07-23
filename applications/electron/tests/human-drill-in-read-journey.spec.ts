import { launchWithMockGh } from './helpers/launch'
import { atpPath } from './helpers/atp'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'
import { expect, test } from '@playwright/test'

test.describe('human-side E2E #476 — direct-action — board drill-in', () => {
  test('clicking a story row opens the drill-in panel with title + body', async () => {
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

      const titleMarker = 'drill-in-target-marker'
      const createResp = await rpc(
        'create_story',
        {
          title: `[Feature] ${titleMarker}`,
          user_job:
            'When a human clicks a story row, the drill-in panel opens with the story title visible.',
          observation_falsifier: 'reject if drill-in panel does not appear after row click',
          acceptance_test_path: atpPath(__filename)
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')

      await expect(board.storyNavigate(1)).toHaveAccessibleName(new RegExp(titleMarker))

      consoleErrors.length = 0
      pageErrors.length = 0

      await board.openStory(1)

      await expect(drill.panel()).toBeVisible()
      await expect(drill.panel()).toContainText(titleMarker)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('human-side E2E #476 — read-class — drill-in body content', () => {
  test('drill-in renders the story user_job + observation_falsifier markers from the body', async () => {
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

      const userJobMarker = 'USER_JOB_DRILL_IN_BODY_BPF12'
      const falsifierMarker = 'FALSIFIER_DRILL_IN_BODY_BPF12'

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] drill-body-target',
          user_job: userJobMarker,
          observation_falsifier: falsifierMarker,
          acceptance_test_path: atpPath(__filename)
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.storyNavigate(1)).toHaveAccessibleName(/drill-body-target/)

      await board.openStory(1)
      await expect(drill.panel()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await expect(drill.panel()).toContainText(userJobMarker, { timeout: 10000 })
      await expect(drill.panel()).toContainText(falsifierMarker)

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('human-side E2E #476 — direct-action — drill-in close', () => {
  test('Escape closes the drill-in panel and returns to board', async () => {
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

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] drill-close-target',
          user_job: 'When the human presses Escape in a drill-in, the drill-in closes.',
          observation_falsifier: 'reject if Escape does not close the drill-in panel',
          acceptance_test_path: atpPath(__filename)
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.storyNavigate(1)).toHaveAccessibleName(/drill-close-target/)

      await board.openStory(1)
      await expect(drill.panel()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      await win.keyboard.press('Escape')
      await expect(drill.panel()).toHaveCount(0)
      await expect(board.storyNavigate(1)).toBeVisible()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

test.describe('human-side E2E #476 — direct-action — drill-in comment input', () => {
  test('comment send button is disabled when input is empty, enabled when typed', async () => {
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

      const createResp = await rpc(
        'create_story',
        {
          title: '[Feature] comment-input-target',
          user_job:
            'When the human opens a drill-in, the comment send button should be disabled until they type something.',
          observation_falsifier:
            'reject if comment send button is enabled with empty input OR remains disabled after typing',
          acceptance_test_path: atpPath(__filename)
        },
        'pm'
      )
      expect(createResp.status).toBe(200)

      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await expect(board.storyNavigate(1)).toHaveAccessibleName(/comment-input-target/)

      await board.openStory(1)
      await expect(drill.panel()).toBeVisible()

      consoleErrors.length = 0
      pageErrors.length = 0

      const sendButton = drill.commentSend()
      await expect(sendButton).toBeVisible()
      await expect(sendButton).toBeDisabled()

      const commentInput = drill.commentInput()
      await commentInput.fill('Some non-trivial comment text.')
      await expect(sendButton).toBeEnabled()

      await commentInput.fill('')
      await expect(sendButton).toBeDisabled()

      await commentInput.fill('   ')
      await expect(sendButton).toBeDisabled()

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
