import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const ITERATION_NUMBER = 999

const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    {
      number: ITERATION_NUMBER,
      title: 'Iteration DeskCheck366',
      state: 'in-progress',
      labels: ['iteration'],
      assignees: [],
      created_at: '2026-04-22T16:00:00Z',
      body: '',
      is_current: true
    },
    {
      number: 366,
      title: 'Comment quote-excerpt',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Quote-excerpt feature story body.',
      iteration_membership: { iteration_number: ITERATION_NUMBER, position: 1 }
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
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/366/comments', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_COMMENTS)
    })
  })
  await win.reload()
  return { app }
}

const openStoryDrillIn = async (
  win: Awaited<ReturnType<Awaited<ReturnType<typeof electron.launch>>['firstWindow']>>
): Promise<void> => {
  await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
  const storyButton = win.locator('[data-test="board-tabpanel-now-content"] [data-test="board-story-366"] .board-story-navigate')
  await storyButton.click()
  await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()
  await expect(win.locator('[data-test="drill-in-comment"]').first()).toBeVisible()
}

test.describe('desk-check #366 comment quote-excerpt', () => {
  test('Cmd+E on selected comment text prepends blockquote to draft', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await openStoryDrillIn(win)

    const firstComment = win.locator('[data-test="drill-in-comment"]').first()
    await expect(firstComment).toHaveAttribute('data-comment-id', '7700')
    await expect(firstComment).toHaveAttribute('data-comment-author', 'designer')

    // Select a substring of the first comment's body via the DOM
    // Selection API — more precise than chord-dragging and doesn't
    // depend on font metrics.
    await win.evaluate(() => {
      const li = document.querySelector<HTMLElement>('[data-comment-id="7700"]')
      if (!li) throw new Error('comment li not found')
      const body = li.querySelector<HTMLElement>('.drill-in-comment-body p')
      if (!body || !body.firstChild) throw new Error('comment body not found')
      const text = body.firstChild as Text
      const range = document.createRange()
      // Grab 'middle-ground proposal' (chars 4-26 of the text)
      range.setStart(text, 4)
      range.setEnd(text, 26)
      const sel = window.getSelection()
      if (!sel) throw new Error('no selection api')
      sel.removeAllRanges()
      sel.addRange(range)
    })

    // Cmd+E / Ctrl+E on the drill-in article — the shortcut is
    // handled by handleKeyDown which delegates to handleQuoteShortcut.
    await win.locator('[data-test="board-drill-in"]').focus()
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+E' : 'Control+E')

    // Draft should now contain the quote-block prefix.
    const draft = await win.locator('[data-test="drill-in-comment-input"]').inputValue()
    console.log(`#366 draft after quote:\n${draft}`)

    expect(draft).toContain('> [@designer](#comment-7700):')
    expect(draft).toContain('> middle-ground proposal')
    // Ends with a blank line (two newlines) before the cursor position.
    expect(draft.endsWith('\n\n')).toBe(true)

    // Textarea should have focus so the user can type directly.
    const focused = await win.evaluate(() => {
      const ta = document.querySelector<HTMLTextAreaElement>('[data-test="drill-in-comment-input"]')
      return document.activeElement === ta
    })
    expect(focused).toBe(true)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '70-quote-shortcut-populates-draft.png')
    })

    await app.close()
  })

  test('Cmd+E without selection is a no-op', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await openStoryDrillIn(win)

    // No selection present (fresh drill-in). Cmd+E should not modify
    // the draft.
    await win.locator('[data-test="board-drill-in"]').focus()
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+E' : 'Control+E')

    const draft = await win.locator('[data-test="drill-in-comment-input"]').inputValue()
    expect(draft).toBe('')

    await app.close()
  })

  test('Cmd+E on selection outside a comment does not quote', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await openStoryDrillIn(win)

    // Select the drill-in title text (which is NOT inside a comment).
    await win.evaluate(() => {
      const title = document.querySelector<HTMLElement>(
        '[data-test="board-drill-in"] h1, [data-test="board-drill-in"] h2, [data-test="board-drill-in"] .drill-in-title'
      )
      if (!title || !title.firstChild) return
      const range = document.createRange()
      range.selectNodeContents(title)
      const sel = window.getSelection()
      if (!sel) return
      sel.removeAllRanges()
      sel.addRange(range)
    })

    await win.locator('[data-test="board-drill-in"]').focus()
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+E' : 'Control+E')

    const draft = await win.locator('[data-test="drill-in-comment-input"]').inputValue()
    expect(draft).toBe('')

    await app.close()
  })
})
