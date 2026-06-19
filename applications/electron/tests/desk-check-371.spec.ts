import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { commentJson } from './helpers/mock_gh'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

// Playwright can't interact with native Electron Menus — menu.popup()
// is an OS-level affordance that lives outside the browser DOM. So
// this test exercises the IPC contract directly: simulate the
// main-process "Quote in reply" click by sending the same IPC frame
// (`bpmux:quote-comment`) the real menu would fire, and assert the
// renderer's subscriber inserts the quote block.

const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck371', '2026-04-22T16:00:00Z'),
    {
      number: 371,
      title: 'Right-click quote-excerpt',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Right-click quote feature story.',
      ...inMockCurrentIteration(1)
    }
  ]
}

const MOCK_COMMENTS = [
  {
    id: 4200,
    author: 'designer',
    body: 'native menu preserves Copy/Paste/Inspect alongside Quote in reply.',
    created_at: '2026-04-22T15:20:00Z'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-371-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-371-'))
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
    match: /\/repos\/acme\/bpmux\/issues\/371\/comments/,
    bodyJson: MOCK_COMMENTS.map((c) =>
      commentJson(c.body, { id: c.id, author: c.author, created_at: c.created_at })
    )
  })
  await win.route('**/api/stories/371/timeline', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: '[]'
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #371 right-click quote context menu', () => {
  test('simulated Quote-in-reply IPC inserts blockquote into draft', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const drillIn = new DrillInPage(win)
    await new BoardPage(win).storyNavigate(371).click()
    await expect(drillIn.panel()).toBeVisible()
    await expect(drillIn.commentItems().first()).toBeVisible()

    // Simulate the main-process click by sending the IPC frame the
    // native "Quote in reply" menu item would emit. No need to
    // actually right-click; we're verifying the renderer half of the
    // contract.
    await app.evaluate(async ({ BrowserWindow }) => {
      const windows = BrowserWindow.getAllWindows()
      if (windows.length === 0) throw new Error('no window')
      windows[0].webContents.send('bpmux:quote-comment', {
        commentId: '4200',
        author: 'designer',
        text: 'native menu preserves Copy/Paste/Inspect alongside Quote in reply.'
      })
    })

    // Draft should contain the quote block.
    const draft = await drillIn.commentInput().inputValue()
    expect(draft).toContain('> [@designer](#comment-4200):')
    expect(draft).toContain('> native menu preserves Copy/Paste/Inspect alongside Quote in reply.')
    expect(draft.endsWith('\n\n')).toBe(true)

    // Textarea should be focused with cursor at prefix.length.
    const focused = await drillIn.commentInput().evaluate((ta) => document.activeElement === ta)
    expect(focused).toBe(true)

    await app.close()
  })
})
