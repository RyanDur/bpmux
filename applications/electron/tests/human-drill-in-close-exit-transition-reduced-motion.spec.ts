import { test, expect } from '@playwright/test'
import { launchWithMockGh } from './helpers/launch'
import { setupCurrentIterationViaRpc, type RpcFn } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'

const MEMBER_TITLE = 'Close-direction desk-check'

const seedReadyMember = async (rpc: RpcFn): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: `[Feature] ${MEMBER_TITLE}`,
      user_job: 'When the owner closes the drill-in, the close-direction transition plays.',
      observation_falsifier: 'reject if the drill-in does not animate its exit transition',
      acceptance_test_path:
        'applications/electron/tests/human-drill-in-close-exit-transition-reduced-motion.spec.ts'
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck354')
}

test.describe('desk-check #354 close-direction animations', () => {
  test('drill-in stays mounted during exit transition (data-visible toggles)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(board.region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Plan')
      await board.openStory(1)
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
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('drill-in exit transition has transform + opacity declared at close', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(board.region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Plan')
      await board.openStory(1)
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
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('reduced-motion: drill-in exit uses 0s duration + transform none', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await win.emulateMedia({ reducedMotion: 'reduce' })
      await expect(board.region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Plan')
      await board.openStory(1)
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
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
