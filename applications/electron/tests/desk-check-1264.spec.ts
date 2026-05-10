import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const ICEBOX_BOARD_FIXTURE = {
  fetched_at: '2026-05-10T20:00:00Z',
  stories: [
    {
      number: 1100,
      title: 'Story above marker A',
      state: 'icebox',
      labels: ['Feature', 'icebox'],
      assignees: [],
      created_at: '2026-05-10T20:00:00Z',
      body: ''
    },
    {
      number: 1200,
      title: 'Story above marker A 2',
      state: 'icebox',
      labels: ['Feature', 'icebox'],
      assignees: [],
      created_at: '2026-05-10T20:00:00Z',
      body: ''
    },
    {
      number: 1300,
      title: 'Story between marker A and B',
      state: 'icebox',
      labels: ['Feature', 'icebox'],
      assignees: [],
      created_at: '2026-05-10T20:00:00Z',
      body: ''
    }
  ],
  icebox_unified: [
    { kind: 'story', number: 1100 },
    { kind: 'story', number: 1200 },
    { kind: 'marker', id: 7 },
    { kind: 'story', number: 1300 },
    { kind: 'marker', id: 8 }
  ]
}

const ITERATION_MARKERS_FIXTURE = [
  { id: 7, theme_name: 'Andromeda', position: 0, created_at: '2026-05-10T20:00:00Z' },
  { id: 8, theme_name: 'Lyra', position: 1, created_at: '2026-05-10T20:00:00Z' }
]

async function launchWithIceboxFixture() {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-1264-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-1264-'))
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
  return { app, win, udd }
}

test.describe('desk-check #1264 per-marker collapse', () => {
  test('icebox renders each iteration as <details>/<summary> per HTML5 native disclosure', async () => {
    const { app, win } = await launchWithIceboxFixture()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await win.locator('[data-test="board-group-icebox"]').click()

    const groupA = win.locator('[data-test="iteration-group-7"]')
    const groupB = win.locator('[data-test="iteration-group-8"]')
    await expect(groupA).toHaveJSProperty('tagName', 'DETAILS')
    await expect(groupB).toHaveJSProperty('tagName', 'DETAILS')

    const summaryA = groupA.locator(':scope > summary')
    await expect(summaryA).toHaveJSProperty('tagName', 'SUMMARY')
    await expect(summaryA).toHaveText(/Andromeda/)

    await app.close()
  })

  test('summary shows count + iteration label', async () => {
    const { app, win } = await launchWithIceboxFixture()
    await win.locator('[data-test="board-group-icebox"]').click()

    const countA = win.locator('[data-test="iteration-marker-7-count"]')
    await expect(countA).toHaveText('2 stories')
    const countB = win.locator('[data-test="iteration-marker-8-count"]')
    await expect(countB).toHaveText('1 story')

    await app.close()
  })

  test('clicking summary toggles collapse + expand', async () => {
    const { app, win } = await launchWithIceboxFixture()
    await win.locator('[data-test="board-group-icebox"]').click()

    const groupA = win.locator('[data-test="iteration-group-7"]')
    await expect(groupA).toHaveJSProperty('open', true)

    await groupA.locator(':scope > summary').click()
    await expect(groupA).toHaveJSProperty('open', false)

    await groupA.locator(':scope > summary').click()
    await expect(groupA).toHaveJSProperty('open', true)

    await app.close()
  })

  test('collapse state persists across page reload', async () => {
    const { app, win } = await launchWithIceboxFixture()
    await win.locator('[data-test="board-group-icebox"]').click()

    const groupA = win.locator('[data-test="iteration-group-7"]')
    await groupA.locator(':scope > summary').click()
    await expect(groupA).toHaveJSProperty('open', false)

    await win.reload()
    await win.locator('[data-test="board-group-icebox"]').click()
    const groupAAfter = win.locator('[data-test="iteration-group-7"]')
    await expect(groupAAfter).toHaveJSProperty('open', false)

    await app.close()
  })
})
