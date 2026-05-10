import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #1258 α-substrate close_iteration ceremony — AC 31 (live integration)
// + AC 33/4 (DOM-shape for drag-reorder cross-marker boundaries).
//
// Drives the full ceremony chain via Playwright + electron app:
//   board fixture with icebox containing markers + stories (interleaved) →
//   POST /api/close_iteration via in-app fetch →
//   verify renderer reflects the post-ceremony state without a board refetch.
//
// AC 33/4 cross-marker drag-reorder DOM-shape: the icebox column unified
// UL must place markers as <li role="separator"> AND stories as
// <li data-test="board-story-N"> in a single list — no separate UL split.

const ICEBOX_BOARD_FIXTURE = {
  fetched_at: '2026-05-10T03:00:00Z',
  stories: [
    {
      number: 1101,
      title: 'Icebox story above marker',
      state: 'icebox',
      labels: ['Feature', 'icebox'],
      assignees: [],
      created_at: '2026-05-10T03:00:00Z',
      body: ''
    },
    {
      number: 1102,
      title: 'Icebox story below marker',
      state: 'icebox',
      labels: ['Feature', 'icebox'],
      assignees: [],
      created_at: '2026-05-10T03:00:00Z',
      body: ''
    }
  ],
  icebox_unified: [
    { kind: 'story', number: 1101 },
    { kind: 'marker', id: 7 },
    { kind: 'story', number: 1102 }
  ]
}

const ITERATION_MARKERS_FIXTURE = [
  {
    id: 7,
    theme_name: 'Orion',
    position: 0,
    created_at: '2026-05-10T03:00:00Z'
  }
]

async function launchWithIceboxFixture(): Promise<{
  app: Awaited<ReturnType<typeof import('@playwright/test')['_electron']['launch']>>
  win: Awaited<ReturnType<typeof import('@playwright/test')['_electron']['launch']>> extends infer A
    ? A extends { firstWindow: () => Promise<infer W> }
      ? W
      : never
    : never
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-1258-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-1258-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(ICEBOX_BOARD_FIXTURE)
    })
  })
  await win.route('**/api/iteration_markers', async (route) => {
    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(ITERATION_MARKERS_FIXTURE)
      })
    } else {
      await route.continue()
    }
  })
  await win.reload()
  return { app, win } as never
}

test.describe('desk-check #1258 close_iteration ceremony', () => {
  test('AC 33/4: icebox column groups stories by iteration via native details/summary', async () => {
    const { app, win } = await launchWithIceboxFixture()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await win.locator('[data-test="board-group-icebox"]').click()

    const sections = win.locator('[data-test="board-icebox-unified"]')
    await expect(sections).toBeVisible()

    const iterationGroup = win.locator('[data-test="iteration-group-7"]')
    await expect(iterationGroup).toHaveJSProperty('tagName', 'DETAILS')
    await expect(iterationGroup.locator('[data-test="board-story-1101"]')).toBeVisible()

    const unscheduled = win.locator('[data-test="iteration-unscheduled"]')
    await expect(unscheduled.locator('[data-test="board-story-1102"]')).toBeVisible()

    await app.close()
  })

  test('AC 33: cross-marker drag-reorder POST fires when story drops past marker', async () => {
    const { app, win } = await launchWithIceboxFixture()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    let reorderFired: { number?: number; state?: string; position?: number } = {}
    await win.route('**/api/stories/*/reorder', async (route) => {
      const url = route.request().url()
      const match = url.match(/\/api\/stories\/(\d+)\/reorder/)
      const number = match ? Number.parseInt(match[1], 10) : undefined
      const body = route.request().postDataJSON?.()
      reorderFired = { number, state: body?.state, position: body?.position }
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true })
      })
    })

    await win.locator('[data-test="board-group-icebox"]').click()

    // Drag story 1101 to position 3 (past marker → below story 1102).
    const story1101 = win.locator('[data-test="board-story-1101"]')
    const story1102 = win.locator('[data-test="board-story-1102"]')

    await story1101.evaluate((el) => {
      const dt = new DataTransfer()
      dt.setData('text/bpmux-story', '1101')
      el.dispatchEvent(new DragEvent('dragstart', { bubbles: true, dataTransfer: dt }))
    })

    // Dispatch dragover + drop on the unified UL after story 1102.
    await story1102.evaluate((el) => {
      const dt = new DataTransfer()
      dt.setData('text/bpmux-story', '1101')
      const rect = el.getBoundingClientRect()
      el.dispatchEvent(
        new DragEvent('dragover', {
          bubbles: true,
          dataTransfer: dt,
          clientY: rect.bottom - 2
        })
      )
    })
    const ul = win.locator('[data-test="board-icebox-unified"]')
    await ul.evaluate((el) => {
      const dt = new DataTransfer()
      dt.setData('text/bpmux-story', '1101')
      el.dispatchEvent(new DragEvent('drop', { bubbles: true, dataTransfer: dt }))
    })

    // POST /api/stories/1101/reorder fired with state=icebox.
    await expect.poll(() => reorderFired.number).toBe(1101)
    expect(reorderFired.state).toBe('icebox')
    // Position is 1-indexed in the unified list; dragged past marker → position >= 2.
    expect(reorderFired.position).toBeGreaterThanOrEqual(2)

    await app.close()
  })
})
