import { test, expect } from '@playwright/test'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { commentJson } from './helpers/mock_gh'
import { setupCurrentIterationViaRpc, type RpcFn } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'
import type { Page } from '@playwright/test'
import type { MockGhServer } from './helpers/mock_gh'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MEMBER_TITLE = 'Comment quote-excerpt'

const MOCK_COMMENTS = [
  {
    id: 7700,
    author: 'designer',
    body: 'The middle-ground proposal is selection-based quoting.',
    created_at: '2026-04-22T15:20:00Z'
  },
  {
    id: 7701,
    author: 'pm',
    body: 'I think concurrent-write disambiguation is the real win here.',
    created_at: '2026-04-22T15:30:00Z'
  }
]

const seedFixture = async (rpc: RpcFn, mockGh: MockGhServer): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: `[Feature] ${MEMBER_TITLE}`,
      user_job: 'When a human selects comment text, Cmd+E prepends a blockquote to the draft.',
      observation_falsifier: 'reject if Cmd+E on a selected comment does not quote into the draft',
      acceptance_test_path:
        'applications/electron/tests/human-drill-in-comment-quote-shortcut.spec.ts'
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck366')
  mockGh.on({
    method: 'GET',
    match: /\/repos\/acme\/bpmux\/issues\/1\/comments/,
    bodyJson: MOCK_COMMENTS.map((c) =>
      commentJson(c.body, { id: c.id, author: c.author, created_at: c.created_at })
    )
  })
}

const openStoryDrillIn = async (win: Page): Promise<void> => {
  const board = new BoardPage(win)
  const drillIn = new DrillInPage(win)
  await expect(new ChatPage(win).region()).toBeVisible()
  await board.openTab('Plan')
  await board.openStory(1)
  await expect(drillIn.panel()).toBeVisible()
  await expect(drillIn.commentItems().first()).toBeVisible()
}

test.describe('desk-check #366 comment quote-excerpt', () => {
  test('Cmd+E on selected comment text prepends blockquote to draft', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const drillIn = new DrillInPage(win)
      await expect(new BoardPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await openStoryDrillIn(win)

      const firstComment = drillIn.commentItems().first()
      await expect(firstComment).toContainText('designer')
      await expect(firstComment).toContainText(
        'The middle-ground proposal is selection-based quoting.'
      )

      await firstComment.locator('.drill-in-comment-body p').evaluate((body: HTMLElement) => {
        if (!body.firstChild) throw new Error('comment body not found')
        const text = body.firstChild as Text
        const range = document.createRange()
        range.setStart(text, 4)
        range.setEnd(text, 26)
        const sel = window.getSelection()
        if (!sel) throw new Error('no selection api')
        sel.removeAllRanges()
        sel.addRange(range)
      })

      await drillIn.panel().focus()
      await win.keyboard.press(process.platform === 'darwin' ? 'Meta+E' : 'Control+E')

      const draft = await drillIn.commentInput().inputValue()
      console.log(`#366 draft after quote:\n${draft}`)

      expect(draft).toContain('> [@designer](#comment-7700):')
      expect(draft).toContain('> middle-ground proposal')
      expect(draft.endsWith('\n\n')).toBe(true)

      await expect(drillIn.commentInput()).toBeFocused()

      await win.screenshot({
        path: join(EVIDENCE_DIR, '70-quote-shortcut-populates-draft.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Cmd+E without selection is a no-op', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const drillIn = new DrillInPage(win)
      await expect(new BoardPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await openStoryDrillIn(win)

      await drillIn.panel().focus()
      await win.keyboard.press(process.platform === 'darwin' ? 'Meta+E' : 'Control+E')

      const draft = await drillIn.commentInput().inputValue()
      expect(draft).toBe('')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('Cmd+E on selection outside a comment does not quote', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const drillIn = new DrillInPage(win)
      await expect(new BoardPage(win).region()).toBeVisible()
      await seedFixture(rpc, mockGh)
      await win.reload()
      await openStoryDrillIn(win)

      await drillIn.title().evaluate((title: HTMLElement) => {
        if (!title.firstChild) return
        const range = document.createRange()
        range.selectNodeContents(title)
        const sel = window.getSelection()
        if (!sel) return
        sel.removeAllRanges()
        sel.addRange(range)
      })

      await drillIn.panel().focus()
      await win.keyboard.press(process.platform === 'darwin' ? 'Meta+E' : 'Control+E')

      const draft = await drillIn.commentInput().inputValue()
      expect(draft).toBe('')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
