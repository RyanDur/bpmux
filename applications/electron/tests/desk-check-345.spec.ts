import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    {
      number: 345,
      title: 'Desk-check story for #345 live comment refresh',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Body for #345 desk-check.'
    }
  ]
}

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

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-345-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-345-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/345/comments', async (route) => {
    if (route.request().method() === 'POST') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true })
      })
    } else {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(INITIAL_COMMENTS)
      })
    }
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #345 live comment refresh', () => {
  test('drill-in opens + comments hydrate on mount', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()
    await win.locator('[data-test="board-story-345"] .board-story-navigate').click()
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()
    await win.waitForTimeout(500)

    const drillIn = win.locator('[data-test="board-drill-in"]')
    await drillIn.screenshot({
      path: join(EVIDENCE_DIR, '30-drillin-with-comments.png')
    })

    const comments = win.locator('.drill-in-comment')
    const count = await comments.count()
    console.log(`#345 AC comments on mount: ${count} rendered`)
    for (let i = 0; i < count; i++) {
      const text = (await comments.nth(i).textContent())?.trim()
      console.log(`  comment[${i}]: "${(text ?? '').replace(/\s+/g, ' ').slice(0, 80)}"`)
    }
    await app.close()
  })

  test('optimistic render: typed comment appears immediately on submit', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await win.locator('[data-test="board-story-345"] .board-story-navigate').click()
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()
    await win.waitForTimeout(500)

    // Type in the comment textarea.
    const textarea = win.locator('[data-test="drill-in-comment-input"]')
    await textarea.fill('Optimistic comment from the desk-check walk.')

    // Count before submit.
    const before = await win.locator('.drill-in-comment').count()
    console.log(`#345 optimistic: ${before} comments before submit`)

    await win.locator('[data-test="drill-in-comment-send"]').click()

    // Optimistic should add immediately (before server round trip).
    await win.waitForTimeout(150)
    const afterImmediate = await win.locator('.drill-in-comment').count()
    console.log(`  immediately after submit: ${afterImmediate} comments (expect before+1)`)

    // Draft should clear.
    const draftAfter = await textarea.inputValue()
    console.log(`  draft after submit: "${draftAfter}" (expect empty)`)

    // Screenshot the optimistic state.
    await win.locator('[data-test="board-drill-in"]').screenshot({
      path: join(EVIDENCE_DIR, '31-drillin-optimistic-add.png')
    })

    await app.close()
  })

  test('rollback: POST failure restores draft + toast fires', async () => {
    const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-345-fail-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-345-fail-'))
    const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
    const win = await app.firstWindow()
    await win.route('**/api/board', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(MOCK_BOARD)
      })
    })
    await win.route('**/api/stories/345/comments', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({ status: 500, body: 'server error' })
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(INITIAL_COMMENTS)
        })
      }
    })
    await win.reload()

    await win.locator('[data-test="board-story-345"] .board-story-navigate').click()
    await expect(win.locator('[data-test="board-drill-in"]')).toBeVisible()
    await win.waitForTimeout(500)

    const textarea = win.locator('[data-test="drill-in-comment-input"]')
    const draft = 'This one will fail — draft should restore.'
    await textarea.fill(draft)
    await win.locator('[data-test="drill-in-comment-send"]').click()
    await win.waitForTimeout(500)

    const draftAfter = await textarea.inputValue()
    console.log(`#345 rollback: draft after 500 error: "${draftAfter}" (expect restored)`)

    const toast = win.locator('[data-test="drill-in-toast"]')
    const toastVisible = await toast.isVisible()
    console.log(`  toast visible: ${toastVisible}`)
    if (toastVisible) {
      const toastText = (await toast.textContent())?.trim()
      console.log(`  toast text: "${toastText}"`)
    }

    await win.locator('[data-test="board-drill-in"]').screenshot({
      path: join(EVIDENCE_DIR, '32-drillin-rollback-toast.png')
    })

    await app.close()
  })
})
