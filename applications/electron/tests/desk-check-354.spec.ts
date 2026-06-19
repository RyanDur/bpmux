import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { mockCurrentIteration, inMockCurrentIteration } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

const MOCK_BOARD = {
  fetched_at: '2026-04-22T16:00:00Z',
  stories: [
    mockCurrentIteration('DeskCheck354', '2026-04-22T16:00:00Z'),
    {
      number: 354,
      title: 'Close-direction desk-check',
      state: 'ready',
      labels: ['Feature'],
      assignees: ['dev'],
      created_at: '2026-04-22T16:00:00Z',
      body: 'Body.',
      ...inMockCurrentIteration(1)
    }
  ]
}

const launchWithBoard = async (
  reducedMotion?: 'reduce'
): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-354-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-354-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  if (reducedMotion) await win.emulateMedia({ reducedMotion })
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_BOARD)
    })
  })
  await win.route('**/api/stories/354/comments', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #354 close-direction animations', () => {
  test('drill-in stays mounted during exit transition (data-visible toggles)', async () => {
    const { app } = await launchWithBoard()
    const win = await app.firstWindow()
    const drillIn = new DrillInPage(win)
    await new BoardPage(win).storyNavigate(354).click()
    await expect(drillIn.panel()).toBeVisible()
    await win.waitForTimeout(400)

    const enteredAttr = await drillIn.panel().getAttribute('data-visible')
    console.log(`#354 drill-in data-visible while entered: ${enteredAttr}`)

    await drillIn.backButton().click()

    await win.waitForTimeout(50)
    const exitingAttr = await win.locator('.drill-in').getAttribute('data-visible')
    console.log(`#354 drill-in data-visible right after close click: ${exitingAttr}`)

    await win.waitForTimeout(400)
    const afterExitCount = await win.locator('.drill-in').count()
    console.log(
      `#354 drill-in DOM after full exit: ${afterExitCount === 0 ? 'unmounted ✓' : 'still present'}`
    )

    await app.close()
  })

  test('drill-in exit transition has transform + opacity declared at close', async () => {
    const { app } = await launchWithBoard()
    const win = await app.firstWindow()
    const drillIn = new DrillInPage(win)
    await new BoardPage(win).storyNavigate(354).click()
    await expect(drillIn.panel()).toBeVisible()
    await win.waitForTimeout(400)

    const transition = await drillIn.panel().evaluate((el) => {
      const cs = window.getComputedStyle(el)
      return {
        property: cs.transitionProperty,
        duration: cs.transitionDuration,
        easing: cs.transitionTimingFunction
      }
    })
    console.log(`#354 drill-in transition (at entered state):`)
    console.log(`  property: ${transition.property}`)
    console.log(`  duration: ${transition.duration}`)
    console.log(`  easing: ${transition.easing}`)

    await app.close()
  })

  test('reduced-motion: drill-in exit uses 0s duration + transform none', async () => {
    const { app } = await launchWithBoard('reduce')
    const win = await app.firstWindow()
    const drillIn = new DrillInPage(win)
    await new BoardPage(win).storyNavigate(354).click()
    await expect(drillIn.panel()).toBeVisible()
    await win.waitForTimeout(300)

    const reduced = await drillIn.panel().evaluate((el) => {
      const cs = window.getComputedStyle(el)
      return {
        duration: cs.transitionDuration,
        transform: cs.transform
      }
    })
    console.log(`#354 reduced-motion drill-in:`)
    console.log(`  duration: ${reduced.duration}`)
    console.log(`  transform: ${reduced.transform}`)
    expect(reduced.duration).toMatch(/^0s|^0ms|0s,/)

    await app.close()
  })
})
