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
  test('Epics section renders + expands via native details/summary', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()
    await expect(win.locator('[data-test="board-list"]')).toBeVisible()

    const epics = win.locator('[data-test="board-epics"]')
    await expect(epics).toBeVisible()

    const epic901 = win.locator('[data-test="board-epic-901"]')
    await expect(epic901).toBeVisible()
    await expect(epic901).not.toHaveAttribute('open', '')

    await expect(epic901.locator('.board-epic-counts')).toContainText('2 children')
    await expect(epic901.locator('.board-epic-counts')).toContainText('1✓')

    await epic901.locator('summary').click()
    await expect(epic901).toHaveAttribute('open', '')

    const childLinks = epic901.locator('.board-epic-children button')
    await expect(childLinks).toHaveCount(2)
    await expect(childLinks.first()).toContainText('#902')
    await expect(childLinks.first()).toContainText('Child story under epic — feature')

    await epic901.locator('summary').click()
    await expect(epic901).not.toHaveAttribute('open', '')

    await app.close()
  })

  test('child story shows ↳ #N parent-epic ref in its state lane', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="board-region"]')).toBeVisible()

    const childRow = win.locator('[data-test="board-story-902"]')
    await expect(childRow).toBeVisible()
    await expect(childRow.locator('.board-story-epic-ref')).toContainText('↳ #901')

    const standaloneRow = win.locator('[data-test="board-story-904"]')
    await expect(standaloneRow).toBeVisible()
    await expect(standaloneRow.locator('.board-story-epic-ref')).toHaveCount(0)

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

  test('multiple epics can expand simultaneously (native details no mutual-exclusion)', async () => {
    const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-457-multi-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-457-multi-'))
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
              number: 901,
              title: 'Epic A',
              state: 'in-progress',
              labels: ['Epic'],
              assignees: [],
              created_at: '2026-04-26T00:00:00Z',
              body: ''
            },
            {
              number: 902,
              title: 'Epic B',
              state: 'in-progress',
              labels: ['Epic'],
              assignees: [],
              created_at: '2026-04-26T00:00:00Z',
              body: ''
            }
          ]
        })
      })
    })
    await win.reload()
    const epicA = win.locator('[data-test="board-epic-901"]')
    const epicB = win.locator('[data-test="board-epic-902"]')
    await epicA.locator('summary').click()
    await epicB.locator('summary').click()
    await expect(epicA).toHaveAttribute('open', '')
    await expect(epicB).toHaveAttribute('open', '')
    await app.close()
  })
})
