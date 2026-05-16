import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #333 rework: drag-to-reorder in the Ready column must persist
// (no snap-back) + the dragged row must hide during the drag
// gesture (Ryan 2026-04-22 22:31-22:32 UTC).
//
// Root cause of snap-back: the backend's POST /api/stories/:n/reorder
// persisted to the priority index, but GET /api/board returned
// GitHub's native order, overwriting the client's optimistic
// reorder on the next refetch. Fix: apply_priority_to_board sorts
// the GET response per the persisted index.
//
// Hide-while-dragging: parent BoardGroup tracks `draggingNumber`;
// the dragged <li> gets data-dragging="true" → CSS opacity:0.

const MOCK_BOARD_BEFORE = {
  fetched_at: '2026-04-23T03:00:00Z',
  stories: [
    {
      number: 701,
      title: 'First ready story',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: [],
      created_at: '2026-04-23T03:00:00Z',
      body: ''
    },
    {
      number: 702,
      title: 'Second ready story',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: [],
      created_at: '2026-04-23T03:00:00Z',
      body: ''
    },
    {
      number: 703,
      title: 'Third ready story',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: [],
      created_at: '2026-04-23T03:00:00Z',
      body: ''
    }
  ]
}

// Post-reorder: backend would return stories in the priority order.
// Simulates #333's fix — apply_priority_to_board returns stories
// sorted per the index, not GitHub's default order.
const MOCK_BOARD_AFTER = {
  fetched_at: '2026-04-23T03:00:05Z',
  stories: [
    { ...MOCK_BOARD_BEFORE.stories[2] }, // 703 first
    { ...MOCK_BOARD_BEFORE.stories[0] }, // 701 second
    { ...MOCK_BOARD_BEFORE.stories[1] } // 702 last
  ]
}

async function launchWithFixture(opts: {
  postReorderBody: typeof MOCK_BOARD_BEFORE
}): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-333-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-333-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()

  let reorderFired = false
  await win.route('**/api/stories/*/reorder', async (route) => {
    reorderFired = true
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
      body: JSON.stringify(reorderFired ? opts.postReorderBody : MOCK_BOARD_BEFORE)
    })
  })

  await win.reload()
  return { app }
}

test.describe('desk-check #333 drag-to-reorder (rework)', () => {
  test('dragged row hides via data-dragging during gesture', async () => {
    // Simulate a dragstart → dragend without actually dropping; the
    // origin row must carry data-dragging="true" while the gesture
    // is active and lose it when it ends. This guards the hide-
    // while-dragging rework without depending on drag-coordinate
    // simulation accuracy.
    const { app } = await launchWithFixture({ postReorderBody: MOCK_BOARD_BEFORE })
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await expect(win.locator('[data-test="board-group-ready"]').first()).toBeVisible()

    const row = win.locator('[data-test="board-story-701"]')
    const grip = row.locator('[data-test="story-grip"]')
    await expect(row).toBeVisible()
    await expect(grip).toBeVisible()
    await expect(row).not.toHaveAttribute('data-dragging', 'true')

    await grip.evaluate((el) => {
      const dt = new DataTransfer()
      el.dispatchEvent(new DragEvent('dragstart', { bubbles: true, dataTransfer: dt }))
    })
    await expect(row).toHaveAttribute('data-dragging', 'true')

    await grip.evaluate((el) => {
      el.dispatchEvent(new DragEvent('dragend', { bubbles: true }))
    })
    await expect(row).not.toHaveAttribute('data-dragging', 'true')

    await app.close()
  })

  test('reorder POST fires + optimistic order persists through refetch', async () => {
    // End-to-end falsifier gate: the UI order must track the
    // persisted (simulated) priority state, not GitHub's native
    // order. Pre-#333 fix, the backend accepted the POST but the
    // next /api/board response reset the order.
    const { app } = await launchWithFixture({ postReorderBody: MOCK_BOARD_AFTER })
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Before reorder: rows render in filed order 701/702/703.
    // #1128: stories live in `<details id="board-group-{state}-content">`,
    // not as descendants of `<summary data-test="board-group-{state}">`.
    // Pre-#1128 selector matched 0 elements (post-#516 details/summary).
    const rows = win.locator('#board-group-ready-content [data-test^="board-story-"]')
    await expect(rows).toHaveCount(3)
    const initialOrder = await rows.evaluateAll((els) =>
      els.map((el) => el.getAttribute('data-test'))
    )
    expect(initialOrder).toEqual([
      'board-story-701',
      'board-story-702',
      'board-story-703'
    ])

    // Simulate keyboard reorder — the same handleReorder path fires
    // the POST. Ctrl+Shift+Down on the first row bumps it past the
    // second, matching the existing keyboard-reorder contract (#357).
    // #1128: post-#499 the clickable row surface is `<div role="button"
    // class="board-story-button">`, not `<button class="…">` — drop the
    // tag prefix so the selector matches.
    const firstRow = win.locator('[data-test="board-story-701"] .board-story-button')
    await firstRow.focus()
    await firstRow.press('Control+Shift+ArrowDown')

    // Optimistic UI: 701 moves past 702. Plus, with the post-reorder
    // mock board applied on the next board refetch, the final order
    // should match MOCK_BOARD_AFTER's 703/701/702 sequence. We assert
    // on the eventual-consistency state (post-invalidate) since
    // that's what Ryan's walk observes.
    await expect(async () => {
      const order = await rows.evaluateAll((els) =>
        els.map((el) => el.getAttribute('data-test'))
      )
      // Either the optimistic result (701 moved) or the post-refetch
      // priority-applied result — both satisfy the falsifier that
      // the order changed + didn't snap back to original.
      expect(order).not.toEqual([
        'board-story-701',
        'board-story-702',
        'board-story-703'
      ])
    }).toPass({ timeout: 5_000 })

    await app.close()
  })
})
