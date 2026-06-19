import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { commentJson } from './helpers/mock_gh'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck366', '2026-04-22T16:00:00Z'),
    {
      number: 366,
      title: 'Comment quote-excerpt',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Quote-excerpt feature story body.',
      ...inMockCurrentIteration(1)
    }
  ]
}

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

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-366-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-366-'))
  const { app, mockGh } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  mockGh.on({
    method: 'GET',
    match: /\/repos\/acme\/bpmux\/issues\/366\/comments/,
    bodyJson: MOCK_COMMENTS.map((c) =>
      commentJson(c.body, { id: c.id, author: c.author, created_at: c.created_at })
    )
  })
  await win.reload()
  return { app }
}

const openStoryDrillIn = async (
  win: Awaited<ReturnType<Awaited<ReturnType<typeof electron.launch>>['firstWindow']>>
): Promise<void> => {
  const drillIn = new DrillInPage(win)
  await expect(new ChatPage(win).region()).toBeVisible()
  await new BoardPage(win).storyNavigate(366).click()
  await expect(drillIn.panel()).toBeVisible()
  await expect(drillIn.commentItems().first()).toBeVisible()
}

test.describe('desk-check #366 comment quote-excerpt', () => {
  test('Cmd+E on selected comment text prepends blockquote to draft', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const drillIn = new DrillInPage(win)
    await openStoryDrillIn(win)

    const firstComment = drillIn.commentItems().first()
    await expect(firstComment).toContainText('designer')
    await expect(firstComment).toContainText(
      'The middle-ground proposal is selection-based quoting.'
    )

    // Select a substring of the first comment's body via the DOM
    // Selection API — more precise than chord-dragging and doesn't
    // depend on font metrics.
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

    // Cmd+E / Ctrl+E on the drill-in article — the shortcut is
    // handled by handleKeyDown which delegates to handleQuoteShortcut.
    await drillIn.panel().focus()
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+E' : 'Control+E')

    // Draft should now contain the quote-block prefix.
    const draft = await drillIn.commentInput().inputValue()
    console.log(`#366 draft after quote:\n${draft}`)

    expect(draft).toContain('> [@designer](#comment-7700):')
    expect(draft).toContain('> middle-ground proposal')
    // Ends with a blank line (two newlines) before the cursor position.
    expect(draft.endsWith('\n\n')).toBe(true)

    // Textarea should have focus so the user can type directly.
    await expect(drillIn.commentInput()).toBeFocused()

    await win.screenshot({
      path: join(EVIDENCE_DIR, '70-quote-shortcut-populates-draft.png')
    })

    await app.close()
  })

  test('Cmd+E without selection is a no-op', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const drillIn = new DrillInPage(win)
    await openStoryDrillIn(win)

    // No selection present (fresh drill-in). Cmd+E should not modify
    // the draft.
    await drillIn.panel().focus()
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+E' : 'Control+E')

    const draft = await drillIn.commentInput().inputValue()
    expect(draft).toBe('')

    await app.close()
  })

  test('Cmd+E on selection outside a comment does not quote', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const drillIn = new DrillInPage(win)
    await openStoryDrillIn(win)

    // Select the drill-in title text (which is NOT inside a comment).
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

    await app.close()
  })
})
