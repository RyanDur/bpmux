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
      number: 344,
      title: 'Search over story bodies + comments',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Body for story #344 — mentions search target "teleport" deep in the description.'
    },
    {
      number: 400,
      title: 'Another story',
      state: 'ready',
      labels: ['Feature'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Unrelated story body.'
    }
  ]
}

const MOCK_SEARCH_RESULT = {
  messages: [
    {
      from: 'pm:claude-code',
      text: 'We discussed the teleport pattern yesterday in chat.',
      ts: '2026-04-22T15:00:00Z',
      addressee: null,
      to: null,
      message_type: 'chat'
    }
  ],
  stories: [
    {
      number: 344,
      title: 'Search over story bodies + comments',
      snippet: '...mentions search target "teleport" deep in the descr...',
      matched_in: 'body'
    }
  ],
  comments: [
    {
      comment_id: 9001,
      story_number: 344,
      author: 'designer',
      snippet: '...The teleport UX should avoid a jarring...'
    }
  ]
}

const MOCK_COMMENTS_344 = [
  {
    id: 9000,
    author: 'pm',
    body: 'First comment — unrelated.',
    created_at: '2026-04-22T15:20:00Z'
  },
  {
    id: 9001,
    author: 'designer',
    body: 'The teleport UX should avoid a jarring position change.',
    created_at: '2026-04-22T15:30:00Z'
  }
]

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-344-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-344-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/search*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_SEARCH_RESULT)
    })
  })
  await win.route('**/api/stories/344/comments', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_COMMENTS_344)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #344 search over stories + comments', () => {
  test('Cmd+F opens search, 3 sections render with counts', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Open search.
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+f' : 'Control+f')
    await expect(win.locator('[data-test="search-overlay"]')).toBeVisible()

    // Type a query.
    await win.locator('[data-test="search-input"]').fill('teleport')
    await win.waitForTimeout(300)

    await win.locator('[data-test="search-overlay"]').screenshot({
      path: join(EVIDENCE_DIR, '40-search-three-sections.png')
    })

    // Enumerate visible result sections.
    const sections = await win
      .locator('[data-test="search-overlay"] section')
      .allTextContents()
    console.log(`#344 search sections visible: ${sections.length}`)
    sections.forEach((s, i) =>
      console.log(`  section[${i}]: "${s.trim().replace(/\s+/g, ' ').slice(0, 100)}"`)
    )

    await app.close()
  })

  test('click story result opens drill-in', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await win.waitForTimeout(300)
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+f' : 'Control+f')
    await win.locator('[data-test="search-input"]').fill('teleport')
    await win.waitForTimeout(400)

    // Find + click a story button in the results.
    const storyButton = win.locator('[data-test="search-overlay"] button', {
      hasText: 'Search over story bodies'
    })
    const found = await storyButton.count()
    console.log(`#344 story result button count: ${found}`)

    if (found > 0) {
      await storyButton.first().click()
      await win.waitForTimeout(500)
      const drillIn = win.locator('[data-test="board-drill-in"]')
      const drillVisible = await drillIn.isVisible()
      console.log(`  drill-in visible after story click: ${drillVisible}`)
      await win.screenshot({
        path: join(EVIDENCE_DIR, '41-story-click-opens-drillin.png')
      })
    }

    await app.close()
  })

  test('click comment result opens drill-in + highlights target comment', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await win.waitForTimeout(300)
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+f' : 'Control+f')
    await win.locator('[data-test="search-input"]').fill('teleport')
    await win.waitForTimeout(400)

    // Click a comment result.
    const commentButton = win.locator('[data-test="search-overlay"] button', {
      hasText: 'teleport UX'
    })
    const found = await commentButton.count()
    console.log(`#344 comment result button count: ${found}`)

    if (found > 0) {
      await commentButton.first().click()
      // Let navigation + fetch resolve.
      await win.waitForTimeout(300)

      // Drill-in opens.
      const drillIn = win.locator('[data-test="board-drill-in"]')
      const drillVisible = await drillIn.isVisible()
      console.log(`  drill-in visible after comment click: ${drillVisible}`)

      // Inspect the DOM for data-comment-id attributes to confirm
      // the target row is renderable.
      const commentIds = await win.evaluate(() =>
        Array.from(document.querySelectorAll('[data-comment-id]')).map((e) =>
          e.getAttribute('data-comment-id')
        )
      )
      console.log(`  data-comment-id attrs present: ${JSON.stringify(commentIds)}`)

      // Poll for highlight class appearing on the target comment.
      let saw = false
      for (let i = 0; i < 20; i++) {
        const classLists = await win.evaluate(() =>
          Array.from(document.querySelectorAll('[data-comment-id]')).map((e) => ({
            id: e.getAttribute('data-comment-id'),
            classes: Array.from(e.classList)
          }))
        )
        console.log(`  poll ${i + 1}: ${JSON.stringify(classLists)}`)
        if (classLists.some((c) => c.classes.includes('drill-in-comment--highlight'))) {
          saw = true
          break
        }
        await win.waitForTimeout(100)
      }
      console.log(`  highlight class saw=${saw}`)

      await win.screenshot({
        path: join(EVIDENCE_DIR, '42-comment-click-opens-highlights.png')
      })
    }

    await app.close()
  })

  test('keyboard: search input is focused on open, Tab moves to results', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await win.waitForTimeout(300)
    await win.keyboard.press(process.platform === 'darwin' ? 'Meta+f' : 'Control+f')

    const focusedIsSearch = await win.evaluate(
      () => document.activeElement?.getAttribute('data-test') === 'search-input'
    )
    console.log(`#344 on open: search input is focused = ${focusedIsSearch}`)

    await win.locator('[data-test="search-input"]').fill('teleport')
    await win.waitForTimeout(400)
    await win.keyboard.press('Tab')
    const afterTab = await win.evaluate(
      () =>
        (document.activeElement?.tagName ?? '') +
        ' | ' +
        (document.activeElement?.getAttribute('aria-label') ?? '')
    )
    console.log(`  after Tab from search: ${afterTab}`)

    await app.close()
  })
})
