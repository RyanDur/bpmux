import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #337 rework regression: the observation-falsifier from the pickup-
// gate paraphrase: "I would reject if, after opening any drill-in
// with comments, I cannot reach the comments list + post-form by
// natural scroll." This test exercises that falsifier directly.

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// Long body so scroll is required to reach comments — if body fits
// the viewport without scroll, we miss the regression we're
// guarding against. 2000+ chars drives past any plausible viewport.
const LONG_BODY = `
## What

${'Long paragraph forcing scroll. '.repeat(50)}

## Why

${'Additional content that pushes the comments section off-screen. '.repeat(40)}

## Acceptance criteria

${'AC line to add vertical bulk. '.repeat(30)}
`.trim()

const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    {
      number: 337,
      title: 'Drill-in scroll rework fixture',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: LONG_BODY
    }
  ]
}

const MOCK_COMMENTS = [
  {
    id: 9337,
    author: 'designer',
    body: 'A comment that must be reachable via natural scroll.',
    created_at: '2026-04-22T16:10:00Z'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-337-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-337-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/337/comments', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_COMMENTS)
    })
  })
  await win.route('**/api/stories/337/timeline', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: '[]'
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #337 rework: drill-in scroll reaches comments', () => {
  test('comments section is reachable via scroll within drill-in', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Open drill-in.
    const card = win.locator('[data-test="board-story-337"]')
    await card.first().click()
    const drillIn = win.locator('[data-test="board-drill-in"]')
    await expect(drillIn).toBeVisible()

    // Comments list exists in DOM (before #337 fix, this was true
    // but unreachable). Assert existence first.
    const commentsList = win.locator('[data-test="drill-in-comments-list"]')
    await expect(commentsList).toBeAttached({ timeout: 5_000 })

    // Observation-falsifier: "I would reject if comments unreachable
    // by natural scroll." Scroll the drill-in container to the
    // bottom, then check comments are visible in the viewport.
    await drillIn.evaluate((el: HTMLElement) => {
      el.scrollTop = el.scrollHeight
    })

    // After scroll, the comments list should be in viewport.
    await expect(commentsList).toBeInViewport({ timeout: 3_000 })
    // The specific comment text confirms we can read content.
    await expect(
      win.locator('[data-test="drill-in-comment"]', {
        hasText: 'reachable via natural scroll'
      })
    ).toBeInViewport({ timeout: 3_000 })

    // Post-form textarea also reachable (falsifier explicitly names
    // "comments list + post-form").
    await expect(
      win.locator('[data-test="drill-in-comment-input"]')
    ).toBeInViewport({ timeout: 3_000 })

    await win.screenshot({
      path: join(EVIDENCE_DIR, '102-drill-in-comments-reachable.png'),
      fullPage: false
    })

    await app.close()
  })

  test('back-crumb header stays sticky during scroll', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    // Click the post-#499 div-class trigger directly; clicking the
    // outer <li> doesn't fire the open-drill-in handler.
    await win.locator('[data-test="board-story-337"] .board-story-button').click()
    const drillIn = win.locator('[data-test="board-drill-in"]')

    const backButton = win.locator('.drill-in-back')
    await expect(backButton).toBeInViewport()

    await expect(drillIn).toHaveAttribute('data-visible', /.*/)

    await drillIn.evaluate((el: HTMLElement) => {
      el.scrollTop = el.scrollHeight
    })

    await expect(backButton).toBeInViewport({ timeout: 3_000 })
    await app.close()
  })
})
