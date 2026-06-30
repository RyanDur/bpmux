import { test, expect } from '@playwright/test'
import { launchWithMockGh, type MockGhLaunchResult } from './helpers/launch'
import { setupCurrentIterationViaRpc, type RpcFn } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'

const createStory = async (rpc: RpcFn, title: string): Promise<void> => {
  const resp = await rpc(
    'create_story',
    {
      title,
      user_job:
        'When a human drafts a comment and presses Cmd+Enter, the comment posts and the draft clears.',
      observation_falsifier: 'reject if Cmd+Enter does not submit the comment or clear the draft',
      acceptance_test_path: 'applications/electron/tests/desk-check-356.spec.ts'
    },
    'pm'
  )
  expect(resp.status).toBe(200)
}

const seedFixture = async (result: MockGhLaunchResult): Promise<void> => {
  const { rpc } = result
  await createStory(rpc, '[Feature] Story for Cmd+Enter comment submit')
  await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck356')
  const readyResp = await rpc('label_story', { id: 1, label: 'ready' }, 'pm')
  expect(readyResp.status).toBe(200)
}

const openDrillIn = async (result: MockGhLaunchResult): Promise<DrillInPage> => {
  const { win } = result
  const board = new BoardPage(win)
  const drillIn = new DrillInPage(win)
  await expect(new ChatPage(win).region()).toBeVisible()
  await seedFixture(result)
  await win.reload()
  await expect(board.region()).toBeVisible()
  await board.openTab('Plan')
  await board.openStory(1)
  await expect(drillIn.panel()).toBeVisible()
  return drillIn
}

test.describe('desk-check #356 comment Cmd+Enter submit', () => {
  test('Cmd+Enter (or Ctrl+Enter) fires POST + clears draft', async () => {
    const result = await launchWithMockGh()
    const { app, mockGh } = result
    try {
      const drillIn = await openDrillIn(result)

      const ta = drillIn.commentInput()
      await ta.focus()
      const draft = 'Draft via Cmd+Enter path'
      await ta.fill(draft)

      const modifier = process.platform === 'darwin' ? 'Meta' : 'Control'
      await ta.press(`${modifier}+Enter`)

      await expect(drillIn.commentItems().filter({ hasText: draft })).toBeVisible()
      await expect(ta).toHaveValue('')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Shift+Enter still inserts newline (does NOT submit)', async () => {
    const result = await launchWithMockGh()
    const { app, mockGh } = result
    try {
      const drillIn = await openDrillIn(result)

      const ta = drillIn.commentInput()
      await ta.focus()
      await ta.fill('Line 1')
      await ta.press('Shift+Enter')
      await ta.pressSequentially('Line 2')

      await expect(drillIn.commentItems()).toHaveCount(0)
      await expect(ta).toHaveValue(/\n/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
