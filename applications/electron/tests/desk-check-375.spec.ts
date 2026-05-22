import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// Fixture carries one story per (lane × state) intersection so the
// state-aware label transitions can be asserted in a single render.
// #375: DELIVERED + feature should read "awaiting owner", ACCEPTED
// + feature should read "owner-accepted"; DELIVERED + bug/chore
// should read "awaiting team", ACCEPTED + bug/chore should read
// "team-accepted". Rework stays "rework" regardless of state.
const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    {
      number: 900,
      title: 'Delivered feature awaiting owner',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: ''
    },
    {
      number: 901,
      title: 'Delivered chore awaiting team',
      state: 'delivered',
      labels: ['Chore', 'delivered', 'ready for accept'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: ''
    },
    {
      number: 902,
      title: 'Accepted feature',
      state: 'accepted',
      labels: ['Feature', 'accepted'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: ''
    },
    {
      number: 903,
      title: 'Accepted chore',
      state: 'accepted',
      labels: ['Chore', 'accepted'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: ''
    }
  ]
}

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-375-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-375-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/*/comments', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.route('**/api/stories/*/timeline', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.reload()
  return { app }
}

const openGroup = async (
  win: Awaited<ReturnType<Awaited<ReturnType<typeof electron.launch>>['firstWindow']>>,
  state: 'delivered' | 'accepted'
): Promise<void> => {
  // Post-#516: `data-test="board-group-{state}"` lives on `<summary>`,
  // not `<details>`. Pre-#1128 helper read `.open` on summary
  // (undefined) + then `group.locator('summary').first()` looked for a
  // descendant summary inside the summary itself — 0 matches → click
  // hung the test. Click the summary directly so native <details>
  // toggle fires.
  await win.locator(`[data-test="board-tab-${state === 'accepted' ? 'done' : 'now'}"]`).click()
  const summary = win.locator(`[data-test="board-group-${state}"]`).first()
  const isOpen = await summary.evaluate((el: Element) => {
    const details = el.closest('details')
    return details ? (details as HTMLDetailsElement).open : false
  })
  if (!isOpen) {
    await summary.click()
  }
}

test.describe('desk-check #375 state-aware badge labels', () => {
  test('delivered rows use awaiting-tense labels', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await openGroup(win, 'delivered')

    const feat = win.locator('[data-test="board-story-900"] [data-test="story-badge"]')
    await expect(feat).toHaveAttribute('data-badge-kind', 'accepted')
    await expect(feat).toContainText('awaiting owner')

    const chore = win.locator('[data-test="board-story-901"] [data-test="story-badge"]')
    await expect(chore).toHaveAttribute('data-badge-kind', 'team-accepted')
    await expect(chore).toContainText('awaiting team')

    await win.screenshot({
      path: join(EVIDENCE_DIR, '99-badge-state-aware-delivered.png'),
      fullPage: true
    })
    await app.close()
  })

  test('accepted rows use accepted-tense labels', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await openGroup(win, 'accepted')

    const feat = win.locator('[data-test="board-story-902"] [data-test="story-badge"]')
    await expect(feat).toHaveAttribute('data-badge-kind', 'accepted')
    await expect(feat).toContainText('owner-accepted')

    const chore = win.locator('[data-test="board-story-903"] [data-test="story-badge"]')
    await expect(chore).toHaveAttribute('data-badge-kind', 'team-accepted')
    await expect(chore).toContainText('team-accepted')

    await app.close()
  })
})
