import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { commentJson } from './helpers/mock_gh'
import { setupCurrentIterationViaRpc, type RpcFn } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'
import type { MockGhServer } from './helpers/mock_gh'

const MEMBER_TITLE = 'Right-click quote-excerpt'

const MOCK_COMMENTS = [
  {
    id: 4200,
    author: 'designer',
    body: 'native menu preserves Copy/Paste/Inspect alongside Quote in reply.',
    created_at: '2026-04-22T15:20:00Z'
  }
]

const seedFixture = async (rpc: RpcFn, mockGh: MockGhServer): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: `[Feature] ${MEMBER_TITLE}`,
      user_job:
        'When the owner picks Quote-in-reply from the native menu, the renderer inserts a blockquote.',
      observation_falsifier: 'reject if the quote-comment IPC frame does not insert a blockquote',
      acceptance_test_path: 'applications/electron/tests/desk-check-371.spec.ts'
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck371')
  mockGh.on({
    method: 'GET',
    match: /\/repos\/acme\/bpmux\/issues\/1\/comments/,
    bodyJson: MOCK_COMMENTS.map((c) =>
      commentJson(c.body, { id: c.id, author: c.author, created_at: c.created_at })
    )
  })
}

test.describe('desk-check #371 right-click quote context menu', () => {
  test('simulated Quote-in-reply IPC inserts blockquote into draft', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(board.region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await expect(board.region()).toBeVisible()
      await board.openTab('Plan')
      await board.openStory(1)
      await expect(drillIn.panel()).toBeVisible()
      await expect(drillIn.commentItems().first()).toBeVisible()

      await app.evaluate(async ({ BrowserWindow }) => {
        const windows = BrowserWindow.getAllWindows()
        if (windows.length === 0) throw new Error('no window')
        windows[0].webContents.send('bpmux:quote-comment', {
          commentId: '4200',
          author: 'designer',
          text: 'native menu preserves Copy/Paste/Inspect alongside Quote in reply.'
        })
      })

      const draft = await drillIn.commentInput().inputValue()
      expect(draft).toContain('> [@designer](#comment-4200):')
      expect(draft).toContain(
        '> native menu preserves Copy/Paste/Inspect alongside Quote in reply.'
      )
      expect(draft.endsWith('\n\n')).toBe(true)

      const focused = await drillIn.commentInput().evaluate((ta) => document.activeElement === ta)
      expect(focused).toBe(true)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
