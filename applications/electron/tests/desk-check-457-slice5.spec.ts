import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const MOCK_BOARD = {
  fetched_at: '2026-04-26T00:00:00Z',
  stories: [
    {
      number: 901,
      title: 'Epic: substrate consolidation',
      state: 'in-progress',
      labels: ['Epic'],
      assignees: [],
      created_at: '2026-04-26T00:00:00Z',
      body: ''
    },
    {
      number: 902,
      title: 'Child story under epic — feature',
      state: 'ready',
      labels: ['Feature'],
      assignees: [],
      created_at: '2026-04-26T00:00:00Z',
      body: '',
      parent_epic: 901
    },
    {
      number: 903,
      title: 'Child story under epic — accepted feature',
      state: 'accepted',
      labels: ['Feature'],
      assignees: [],
      created_at: '2026-04-26T00:00:00Z',
      body: '',
      parent_epic: 901
    },
    {
      number: 904,
      title: 'Standalone (no epic) story',
      state: 'ready',
      labels: ['Feature'],
      assignees: [],
      created_at: '2026-04-26T00:00:00Z',
      body: ''
    }
  ]
}

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-457-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-457-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #457 slice 5 — Electron Epic-summary section', () => {
  test('#1185 Epic meta-card: progress + breakdown + no nested children + drill-in opens via header button', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()
    await expect(win.locator('[data-test="board-list"]')).toBeVisible()

    const epics = win.locator('[data-test="board-epics"]')
    await expect(epics).toBeVisible()

    const epic901 = win.locator('[data-test="board-epic-901"]')
    await expect(epic901).toBeVisible()

    const progress = win.locator('[data-test="board-epic-901-progress"]')
    await expect(progress).toBeVisible()
    await expect(progress).toHaveAttribute('value', '1')
    await expect(progress).toHaveAttribute('max', '2')

    const miniboard = win.locator('[data-test="board-epic-901-miniboard"]')
    await expect(miniboard).toBeVisible()
    const acceptedCol = win.locator('[data-test="board-epic-901-miniboard-column-accepted"]')
    await expect(acceptedCol).toContainText('(1)')
    const readyCol = win.locator('[data-test="board-epic-901-miniboard-column-ready"]')
    await expect(readyCol).toContainText('(1)')

    await expect(epic901.locator('.board-epic-children')).toHaveCount(0)
    await expect(epic901.locator('.board-epic-child-state')).toHaveCount(0)
    await expect(epic901.locator('.board-story-row')).toHaveCount(0)

    await app.close()
  })

  test('#1260 child story shows ↳ Epic #N link (no full Epic title in visible text) in its state lane', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const childRow = win.locator('[data-test="board-story-902"]')
    await expect(childRow).toBeVisible()
    const epicRef = childRow.locator('[data-test="epic-link"]')
    await expect(epicRef).toHaveText('↳ Epic #901')
    await expect(epicRef).toHaveAttribute('aria-label', /open epic #901 Epic: substrate consolidation/)

    const standaloneRow = win.locator('[data-test="board-story-904"]')
    await expect(standaloneRow).toBeVisible()
    await expect(standaloneRow.locator('[data-test="epic-link"]')).toHaveCount(0)

    await app.close()
  })

  test('no Epics section when board has no Epic stories', async () => {
    const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-457-empty-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-457-empty-'))
    const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
    const win = await app.firstWindow()
    await win.route('**/api/board', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          fetched_at: '2026-04-26T00:00:00Z',
          stories: [
            {
              number: 1001,
              title: 'No epic on this board',
              state: 'ready',
              labels: ['Feature'],
              assignees: [],
              created_at: '2026-04-26T00:00:00Z',
              body: ''
            }
          ]
        })
      })
    })
    await win.reload()
    await expect(win.locator('[data-test="board-list"]')).toBeVisible()
    await expect(win.locator('[data-test="board-epics"]')).toHaveCount(0)
    await app.close()
  })

})
