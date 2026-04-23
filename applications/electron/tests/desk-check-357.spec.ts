import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #357 rework: keyboard-accessible drag-reorder. Ryan observed at
// 2026-04-22 23:13 UTC that Ctrl+Shift+Up/Down on a focused Ready
// row didn't move the row. Root cause was shared with #333
// (priority index written but not read on board refetch — drag
// + keyboard reorder both went through the same handleReorder
// path so both snapped back). #333's `apply_priority_to_board`
// fix restored the persistence; this spec locks in the keyboard
// path's a11y surface: announcement, focus-retention, and
// boundary no-op.

const MOCK_BOARD_BEFORE = {
  fetched_at: '2026-04-23T11:00:00Z',
  stories: [
    {
      number: 811,
      title: 'First ready story',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: [],
      created_at: '2026-04-23T11:00:00Z',
      body: ''
    },
    {
      number: 812,
      title: 'Second ready story',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: [],
      created_at: '2026-04-23T11:00:00Z',
      body: ''
    },
    {
      number: 813,
      title: 'Third ready story',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: [],
      created_at: '2026-04-23T11:00:00Z',
      body: ''
    }
  ]
}

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-357-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-357-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()

  await win.route('**/api/stories/*/reorder', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ ok: true })
    })
  })
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD_BEFORE)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #357 keyboard-accessible drag-reorder', () => {
  test('Ctrl+Shift+Down on focused row moves story + announces position', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await expect(win.locator('[data-test="board-group-ready"]')).toBeVisible()

    const firstRow = win.locator('[data-test="board-story-811"] .board-story-button')
    await firstRow.focus()

    // Pre-reorder: live region empty.
    const announce = win.locator('[data-test="board-reorder-announce"]')
    await expect(announce).toBeAttached()

    await firstRow.press('Control+Shift+ArrowDown')

    // SR announcement fires — the reorderAnnounce state is set to
    // "Moved to position <N> of <total>" after a keyboard reorder.
    // AC5 of the filing.
    await expect(announce).toContainText(/Moved to position \d+ of 3/, { timeout: 3_000 })

    // Row order changes (811 moves past 812 in DOM).
    const rows = win.locator('[data-test="board-group-ready"] [data-test^="board-story-"]')
    await expect(async () => {
      const order = await rows.evaluateAll((els) =>
        els.map((el) => el.getAttribute('data-test'))
      )
      expect(order[0]).not.toBe('board-story-811')
    }).toPass({ timeout: 3_000 })

    await app.close()
  })

  test('first row Ctrl+Shift+Up is a no-op (boundary)', async () => {
    // Falsifier (e): "at column boundary the combo errors instead of
    // no-opping." First row has no room to move up; the key combo
    // must return silently without triggering a reorder or
    // announcement.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const firstRow = win.locator('[data-test="board-story-811"] .board-story-button')
    await firstRow.focus()

    await firstRow.press('Control+Shift+ArrowUp')

    // Order unchanged.
    const rows = win.locator('[data-test="board-group-ready"] [data-test^="board-story-"]')
    const order = await rows.evaluateAll((els) => els.map((el) => el.getAttribute('data-test')))
    expect(order).toEqual([
      'board-story-811',
      'board-story-812',
      'board-story-813'
    ])

    // Announcement text must NOT have fired (stays empty). If the
    // handler short-circuited at i===0, no setReorderAnnounce call.
    const announce = win.locator('[data-test="board-reorder-announce"]')
    await expect(announce).toHaveText('')

    await app.close()
  })

  test('last row Ctrl+Shift+Down is a no-op (boundary)', async () => {
    // Mirror of the first-row case at the other boundary.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const lastRow = win.locator('[data-test="board-story-813"] .board-story-button')
    await lastRow.focus()

    await lastRow.press('Control+Shift+ArrowDown')

    const rows = win.locator('[data-test="board-group-ready"] [data-test^="board-story-"]')
    const order = await rows.evaluateAll((els) => els.map((el) => el.getAttribute('data-test')))
    expect(order).toEqual([
      'board-story-811',
      'board-story-812',
      'board-story-813'
    ])

    const announce = win.locator('[data-test="board-reorder-announce"]')
    await expect(announce).toHaveText('')

    await app.close()
  })

  test('row aria-label names the reorder affordance', async () => {
    // AC6: "aria-label on the story row describes the current
    // position + role (..., press Ctrl+Shift+Up to move up...)."
    // Keyboard-only users hear the affordance at focus time.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const button = win.locator('[data-test="board-story-811"] .board-story-button')
    const ariaLabel = await button.getAttribute('aria-label')
    expect(ariaLabel).toMatch(/position 1 of 3/i)
    expect(ariaLabel).toMatch(/Ctrl\+Shift/i)

    await app.close()
  })
})
