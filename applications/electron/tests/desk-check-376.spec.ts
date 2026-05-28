import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

// #376: verify color distinguishes awaiting vs accepted at the
// CSS computed-style level. A pure DOM/attribute test passes even
// when the visual colors are identical; asserting getComputedStyle
// catches the "same color by accident" regression Ryan observed in
// img_10.
const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck376', '2026-04-22T16:00:00Z'),
    {
      number: 950,
      title: 'delivered feature (awaiting owner)',
      state: 'delivered',
      labels: ['Feature', 'delivered', 'ready for accept'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: '',
      ...inMockCurrentIteration(1)
    },
    {
      number: 951,
      title: 'accepted feature (owner-accepted)',
      state: 'accepted',
      labels: ['Feature', 'accepted'],
      acceptor: 'owner',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: ''
    },
    {
      number: 952,
      title: 'delivered chore (awaiting team)',
      state: 'delivered',
      labels: ['Chore', 'delivered', 'ready for accept'],
      acceptor: 'team',
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: '',
      ...inMockCurrentIteration(2)
    }
  ]
}

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-376-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-376-'))
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
  // Post-#516: data-test on summary, not details. See desk-check-375
  // for full diagnosis. Click summary directly to fire <details>
  // toggle.
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

test.describe('desk-check #376 color-differentiate awaiting vs accepted', () => {
  test('data-state attribute lands on each badge', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await openGroup(win, 'delivered')
    await openGroup(win, 'accepted')

    await expect(
      win.locator('[data-test="board-tabpanel-now-content"] [data-test="board-story-950"] [data-test="story-badge"]')
    ).toHaveAttribute('data-state', 'delivered')

    await expect(
      win.locator('[data-test="board-tabpanel-done-content"] [data-test="board-story-951"] [data-test="story-badge"]')
    ).toHaveAttribute('data-state', 'accepted')

    await expect(
      win.locator('[data-test="board-tabpanel-now-content"] [data-test="board-story-952"] [data-test="story-badge"]')
    ).toHaveAttribute('data-state', 'delivered')
    await app.close()
  })

  test('delivered and accepted badges render in different colors', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await openGroup(win, 'delivered')
    await openGroup(win, 'accepted')

    // Both badges share the same kind ("accepted" = owner-lane
    // outlined); the color axis must differ between states. We
    // read computed styles so "same color" regressions (Ryan's
    // img_10 observation) fail the test — a simpler attribute
    // assertion wouldn't catch a CSS rule deleted.
    const readBorderColor = async (n: number, panel: 'now' | 'done'): Promise<string> =>
      win
        .locator(`[data-test="board-tabpanel-${panel}-content"] [data-test="board-story-${n}"] [data-test="story-badge"]`)
        .evaluate((el: HTMLElement) => getComputedStyle(el).borderTopColor)

    const deliveredColor = await readBorderColor(950, 'now')
    const acceptedColor = await readBorderColor(951, 'done')
    console.log(`#376 delivered badge border: ${deliveredColor}`)
    console.log(`#376 accepted badge border: ${acceptedColor}`)
    expect(deliveredColor).not.toBe(acceptedColor)

    await win.screenshot({
      path: join(EVIDENCE_DIR, '100-badge-colors-differ.png')
    })
    await app.close()
  })
})
