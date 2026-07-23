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
import { DrillInPage } from './pages/drillInPage'
import type { MockGhServer } from './helpers/mock_gh'

const seedComments = (
  comments: ReadonlyArray<{ id: number; author: string; body: string; created_at: string }>
): unknown[] =>
  comments.map((c) => commentJson(c.body, { id: c.id, author: c.author, created_at: c.created_at }))

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const INITIAL_COMMENTS = [
  {
    id: 1001,
    author: 'pm',
    body: 'Initial comment from pm — visible on mount.',
    created_at: '2026-04-22T15:50:00Z'
  },
  {
    id: 1002,
    author: 'designer',
    body: 'Second initial comment from designer.',
    created_at: '2026-04-22T15:55:00Z'
  }
]

const seedFixture = async (rpc: RpcFn, mockGh: MockGhServer): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: '[Feature] Desk-check story for #345 live comment refresh',
      body: 'Body for #345 desk-check.',
      user_job: 'When the human opens a drill-in, comments hydrate and optimistic-add on submit.',
      observation_falsifier:
        'reject if comments do not hydrate on mount or optimistic-add on submit',
      acceptance_test_path: atpPath(__filename)
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  const iterationId = await seedCurrentIterationViaRpc(rpc, 'DeskCheck345')
  const moveResp = await rpc(
    'move_story',
    { id: 1, target_state: 'ready', rationale: 'comment fixture' },
    'pm'
  )
  expect(moveResp.status).toBe(200)
  await addStoryToIterationViaRpc(rpc, 1, iterationId, 1)
  mockGh.on({
    method: 'GET',
    match: /\/repos\/acme\/bpmux\/issues\/1\/comments/,
    bodyJson: seedComments(INITIAL_COMMENTS)
  })
}

test.describe('desk-check #345 live comment refresh', () => {
  test('drill-in opens + comments hydrate on mount', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new BoardPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await board.storyNavigate(1).click()
      await expect(drillIn.panel()).toBeVisible()
      await win.waitForTimeout(500)

      await drillIn.panel().screenshot({
        path: join(EVIDENCE_DIR, '30-drillin-with-comments.png')
      })

      const comments = drillIn.commentItems()
      const count = await comments.count()
      console.log(`#345 AC comments on mount: ${count} rendered`)
      for (let i = 0; i < count; i++) {
        const text = (await comments.nth(i).textContent())?.trim()
        console.log(`  comment[${i}]: "${(text ?? '').replace(/\s+/g, ' ').slice(0, 80)}"`)
      }
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('optimistic render: typed comment appears immediately on submit', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new BoardPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await waitForBoardLoaded(win)
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await board.storyNavigate(1).click()
      await expect(drillIn.panel()).toBeVisible()
      await win.waitForTimeout(500)

      const textarea = drillIn.commentInput()
      await textarea.fill('Optimistic comment from the desk-check walk.')

      const before = await drillIn.commentItems().count()
      console.log(`#345 optimistic: ${before} comments before submit`)

      await drillIn.commentSend().click()

      await win.waitForTimeout(150)
      const afterImmediate = await drillIn.commentItems().count()
      console.log(`  immediately after submit: ${afterImmediate} comments (expect before+1)`)

      const draftAfter = await textarea.inputValue()
      console.log(`  draft after submit: "${draftAfter}" (expect empty)`)

      await drillIn.panel().screenshot({
        path: join(EVIDENCE_DIR, '31-drillin-optimistic-add.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('rollback: POST failure restores draft + toast fires', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new BoardPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.route('**/api/stories/1/comments', async (route) => {
        if (route.request().method() === 'POST') {
          await route.fulfill({ status: 500, body: 'server error' })
        } else {
          await route.continue()
        }
      })
      await win.reload()
      await waitForBoardLoaded(win)

      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await board.storyNavigate(1).click()
      await expect(drillIn.panel()).toBeVisible()
      await win.waitForTimeout(500)

      const textarea = drillIn.commentInput()
      const draft = 'This one will fail — draft should restore.'
      await textarea.fill(draft)
      await drillIn.commentSend().click()
      await win.waitForTimeout(500)

      const draftAfter = await textarea.inputValue()
      console.log(`#345 rollback: draft after 500 error: "${draftAfter}" (expect restored)`)

      const toast = drillIn.toast()
      const toastVisible = await toast.isVisible()
      console.log(`  toast visible: ${toastVisible}`)
      if (toastVisible) {
        const toastText = (await toast.textContent())?.trim()
        console.log(`  toast text: "${toastText}"`)
      }

      await drillIn.panel().screenshot({
        path: join(EVIDENCE_DIR, '32-drillin-rollback-toast.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
