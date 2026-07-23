import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { join } from 'path'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import { commentJson } from './helpers/mock_gh'
import {
  addStoryToIterationViaRpc,
  seedCurrentIterationViaRpc,
  type RpcFn
} from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'
import type { MockGhServer } from './helpers/mock_gh'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const LONG_BODY = `
## What

${'Long paragraph forcing scroll. '.repeat(50)}

## Why

${'Additional content that pushes the comments section off-screen. '.repeat(40)}

## Acceptance criteria

${'AC line to add vertical bulk. '.repeat(30)}
`.trim()

const MOCK_COMMENTS = [
  {
    id: 9337,
    author: 'designer',
    body: 'A comment that must be reachable via natural scroll.',
    created_at: '2026-04-22T16:10:00Z'
  }
]

const seedFixture = async (rpc: RpcFn, mockGh: MockGhServer): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: '[Feature] Drill-in scroll rework fixture',
      body: LONG_BODY,
      user_job:
        'When a human opens a drill-in with comments, scroll reaches the comments + post-form.',
      observation_falsifier:
        'reject if the comments list + post-form cannot be reached by natural scroll',
      acceptance_test_path: atpPath(__filename)
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  const iterationId = await seedCurrentIterationViaRpc(rpc, 'DeskCheck337')
  const moveResp = await rpc(
    'move_story',
    { id: 1, target_state: 'ready', rationale: 'scroll fixture' },
    'pm'
  )
  expect(moveResp.status).toBe(200)
  await addStoryToIterationViaRpc(rpc, 1, iterationId, 1)
  mockGh.on({
    method: 'GET',
    match: /\/repos\/acme\/bpmux\/issues\/1\/comments/,
    bodyJson: MOCK_COMMENTS.map((c) =>
      commentJson(c.body, { id: c.id, author: c.author, created_at: c.created_at })
    )
  })
}

test.describe('desk-check #337 rework: drill-in scroll reaches comments', () => {
  test('comments section is reachable via scroll within drill-in', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)

      await board.storyNavigate(1).click()
      await expect(drillIn.panel()).toBeVisible()

      const commentsList = drillIn.commentList()
      await expect(commentsList).toBeAttached({ timeout: 5_000 })

      await board.boardScrollPanel().evaluate((el: HTMLElement) => {
        el.scrollTop = el.scrollHeight
      })

      await expect(commentsList).toBeInViewport({ timeout: 3_000 })
      await expect(
        drillIn.commentItems().filter({ hasText: 'reachable via natural scroll' })
      ).toBeInViewport({ timeout: 3_000 })

      await expect(drillIn.commentInput()).toBeInViewport({
        timeout: 3_000
      })

      await win.screenshot({
        path: join(EVIDENCE_DIR, '102-drill-in-comments-reachable.png'),
        fullPage: false
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('back-crumb header stays sticky during scroll', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await board.storyNavigate(1).click()
      const panel = drillIn.panel()

      const backButton = drillIn.backButton()
      await expect(backButton).toBeInViewport()

      await expect(panel).toHaveAttribute('data-visible', /.*/)

      await panel.evaluate((el: HTMLElement) => {
        el.scrollTop = el.scrollHeight
      })

      await expect(backButton).toBeInViewport({ timeout: 3_000 })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
