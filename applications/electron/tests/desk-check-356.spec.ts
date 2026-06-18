import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'

const MOCK_BOARD = {
  fetched_at: '2026-04-22T18:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck356', '2026-04-22T16:00:00Z'),
    {
      number: 356,
      title: 'Story for Cmd+Enter comment submit',
      state: 'ready',
      labels: ['Feature'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Body.',
      ...inMockCurrentIteration(1)
    }
  ]
}

const launchBoard = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-356-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-356-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/356/comments', async (route) => {
    if (route.request().method() === 'POST') {
      await route.fulfill({ status: 200, body: '{"ok":true}' })
    } else {
      await route.fulfill({ status: 200, body: '[]' })
    }
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #356 comment Cmd+Enter submit', () => {
  test('Cmd+Enter (or Ctrl+Enter) fires POST + clears draft', async () => {
    const { app } = await launchBoard()
    const win = await app.firstWindow()
    await win
      .locator(
        '[data-test="board-tabpanel-now-content"] [data-test="board-story-356"] .board-story-navigate'
      )
      .click()
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()
    await win.waitForTimeout(500)

    const ta = win.locator('[data-test="drill-in-comment-input"]')
    await ta.focus()
    const draft = 'Draft via Cmd+Enter path'
    await ta.fill(draft)

    // Cmd on mac, Ctrl otherwise — the handler accepts either.
    const modifier = process.platform === 'darwin' ? 'Meta' : 'Control'
    await ta.press(`${modifier}+Enter`)
    await win.waitForTimeout(400)

    await expect(win.locator('[data-test="drill-in-comment"]', { hasText: draft })).toBeVisible()
    await expect(ta).toHaveValue('')

    await app.close()
  })

  test('Shift+Enter still inserts newline (does NOT submit)', async () => {
    const { app } = await launchBoard()
    const win = await app.firstWindow()
    await win
      .locator(
        '[data-test="board-tabpanel-now-content"] [data-test="board-story-356"] .board-story-navigate'
      )
      .click()
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()
    await win.waitForTimeout(500)

    const ta = win.locator('[data-test="drill-in-comment-input"]')
    await ta.focus()
    await ta.fill('Line 1')
    await ta.press('Shift+Enter')
    await ta.pressSequentially('Line 2')
    await win.waitForTimeout(200)

    await expect(win.locator('[data-test="drill-in-comment"]')).toHaveCount(0)
    await expect(ta).toHaveValue(/\n/)

    await app.close()
  })
})
