import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const launchWith = async (
  board: object
): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
  consoleErrors: string[]
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-457-6-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-457-6-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  const consoleErrors: string[] = []
  win.on('console', (m) => {
    if (m.type() === 'error') consoleErrors.push(m.text())
  })
  win.on('pageerror', (e) => consoleErrors.push(String(e)))
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(board)
    })
  })
  await win.reload()
  return { app, consoleErrors }
}

const ts = '2026-04-26T00:00:00Z'

test.describe('desk-check #457 slice 6 — Epic hybrid state-derivation + lane filter', () => {
  test('Epic with in-progress child derives in-progress state', async () => {
    const { app, consoleErrors } = await launchWith({
      fetched_at: ts,
      stories: [
        {
          number: 901,
          title: 'Epic with in-progress work',
          state: 'ready',
          labels: ['Epic'],
          assignees: [],
          created_at: ts,
          body: ''
        },
        {
          number: 902,
          title: 'Child in progress',
          state: 'in-progress',
          labels: ['Feature'],
          assignees: [],
          created_at: ts,
          body: '',
          parent_epic: 901
        }
      ]
    })
    const win = await app.firstWindow()
    const stateBadge = win.locator('[data-test="board-epic-901-state"]')
    await expect(stateBadge).toHaveText('in-progress')
    expect(consoleErrors).toEqual([])
    await app.close()
  })

  test('Epic with all-delivered children derives in-progress (delivered is not a terminal child state per #1450 truth-table)', async () => {
    const { app, consoleErrors } = await launchWith({
      fetched_at: ts,
      stories: [
        {
          number: 911,
          title: 'Epic ready for owner',
          state: 'ready',
          labels: ['Epic'],
          assignees: [],
          created_at: ts,
          body: ''
        },
        {
          number: 912,
          title: 'Child A delivered',
          state: 'delivered',
          labels: ['Feature'],
          assignees: [],
          created_at: ts,
          body: '',
          parent_epic: 911
        },
        {
          number: 913,
          title: 'Child B delivered',
          state: 'delivered',
          labels: ['Feature'],
          assignees: [],
          created_at: ts,
          body: '',
          parent_epic: 911
        }
      ]
    })
    const win = await app.firstWindow()
    const stateBadge = win.locator('[data-test="board-epic-911-state"]')
    await expect(stateBadge).toHaveText('in-progress')
    expect(consoleErrors).toEqual([])
    await app.close()
  })

  test('Epic explicit state is ignored; in-progress child derives in-progress per #1450 truth-table', async () => {
    const { app, consoleErrors } = await launchWith({
      fetched_at: ts,
      stories: [
        {
          number: 921,
          title: 'Owner-accepted epic',
          state: 'accepted',
          labels: ['Epic'],
          assignees: [],
          created_at: ts,
          body: ''
        },
        {
          number: 922,
          title: 'Late-arriving child',
          state: 'in-progress',
          labels: ['Feature'],
          assignees: [],
          created_at: ts,
          body: '',
          parent_epic: 921
        }
      ]
    })
    const win = await app.firstWindow()
    const stateBadge = win.locator('[data-test="board-epic-921-state"]')
    await expect(stateBadge).toHaveText('in-progress')
    expect(consoleErrors).toEqual([])
    await app.close()
  })

  test('Epic story is filtered out of state lanes (lives only in Epics section)', async () => {
    const { app, consoleErrors } = await launchWith({
      fetched_at: ts,
      stories: [
        {
          number: 931,
          title: 'Lone epic in ready',
          state: 'ready',
          labels: ['Epic'],
          assignees: [],
          created_at: ts,
          body: ''
        },
        {
          number: 932,
          title: 'Plain ready feature',
          state: 'ready',
          labels: ['Feature'],
          assignees: [],
          created_at: ts,
          body: ''
        }
      ]
    })
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="board-epic-931"]')).toBeVisible()
    await expect(win.locator('[data-test="board-story-931"]')).toHaveCount(0)
    await expect(win.locator('[data-test="board-story-932"]')).toBeVisible()
    expect(consoleErrors).toEqual([])
    await app.close()
  })
})
